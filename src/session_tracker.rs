use std::collections::HashSet;
use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

use crate::system;

/// Get or create session directory (user-specific, secure)
fn get_session_dir() -> Result<PathBuf, String> {
    // Try XDG_RUNTIME_DIR first (standard for user-specific runtime files)
    let base_dir = env::var("XDG_RUNTIME_DIR")
        .or_else(|_| env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());

    // Use UID for additional security
    let uid = system::get_user_id().map_err(|e| format!("Failed to get user ID: {}", e))?;

    let session_dir = PathBuf::from(format!("{}/whi-{}", base_dir, uid));

    // Create directory with restrictive permissions (0700) if it doesn't exist
    if !session_dir.exists() {
        #[cfg(unix)]
        {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&session_dir)
                .map_err(|e| format!("Failed to create session dir: {}", e))?;
        }

        #[cfg(not(unix))]
        {
            fs::create_dir_all(&session_dir)
                .map_err(|e| format!("Failed to create session dir: {}", e))?;
        }
    }

    Ok(session_dir)
}

/// Get path to session log file for given PID
pub fn get_session_file(pid: u32) -> Result<PathBuf, String> {
    let session_dir = get_session_dir()?;
    Ok(session_dir.join(format!("session_{}.log", pid)))
}

/// Read explicitly affected and deleted paths from the session log
pub fn read_session_paths(pid: u32) -> Result<(HashSet<String>, Vec<String>), String> {
    let session_file = get_session_file(pid)?;

    if !session_file.exists() {
        return Ok((HashSet::new(), Vec::new()));
    }

    let content = fs::read_to_string(&session_file)
        .map_err(|e| format!("Failed to read session log: {e}"))?;

    let mut affected = HashSet::new();
    let mut deleted = Vec::new();

    for line in content.lines() {
        if let Some((op_type, path)) = line.split_once(' ') {
            match op_type {
                "deleted" | "delete" => {
                    // Track deleted paths separately (including duplicates)
                    deleted.push(path.to_string());
                }
                _ => {
                    // Track moved/swapped/preferred paths
                    affected.insert(path.to_string());
                }
            }
        }
    }

    Ok((affected, deleted))
}

/// Write operation with affected paths to session log
pub fn write_operation(pid: u32, op_type: &str, paths: &[String]) -> Result<(), String> {
    if paths.is_empty() {
        return Ok(());
    }

    let session_file = get_session_file(pid)?;

    #[cfg(unix)]
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600) // Restrictive permissions: user read/write only
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log: {e}"))?;

    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log: {e}"))?;

    for path in paths {
        writeln!(file, "{} {}", op_type, path)
            .map_err(|e| format!("Failed to write to session log: {e}"))?;
    }

    Ok(())
}

/// Clear the session log for given PID
pub fn clear_session(pid: u32) -> Result<(), String> {
    let session_file = get_session_file(pid)?;
    if session_file.exists() {
        fs::remove_file(&session_file).map_err(|e| format!("Failed to remove session log: {e}"))?;
    }
    Ok(())
}

/// Get all session files in the session directory
fn get_all_session_files() -> Result<Vec<(PathBuf, std::time::SystemTime)>, String> {
    let session_dir = get_session_dir()?;

    if !session_dir.exists() {
        return Ok(Vec::new());
    }

    let entries =
        fs::read_dir(&session_dir).map_err(|e| format!("Failed to read session directory: {e}"))?;

    let mut session_files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("session_") && name.ends_with(".log") {
                if let Ok(metadata) = entry.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        session_files.push((path, modified));
                    }
                }
            }
        }
    }

    Ok(session_files)
}

/// Cleanup old session files (round robin at >30 files)
/// Returns the number of files cleaned up
pub fn cleanup_old_sessions() -> Result<usize, String> {
    let mut session_files = get_all_session_files()?;

    if session_files.len() <= 30 {
        return Ok(0);
    }

    // Sort by modification time (oldest first)
    session_files.sort_by(|a, b| a.1.cmp(&b.1));

    // Delete oldest files until we have 30 or fewer
    let files_to_delete = session_files.len() - 30;
    let mut deleted_count = 0;

    for (path, _) in session_files.iter().take(files_to_delete) {
        if fs::remove_file(path).is_ok() {
            deleted_count += 1;
        }
    }

    Ok(deleted_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_file_path() {
        let path = get_session_file(12345).unwrap();
        let path_str = path.to_string_lossy();
        assert!(
            path_str.contains("whi-") && path_str.ends_with("session_12345.log"),
            "Session file path should be in user-specific directory: {}",
            path_str
        );
    }

    #[test]
    fn test_session_dir_creation() {
        // Session directory should be created on first access
        let dir = get_session_dir().unwrap();
        assert!(dir.exists(), "Session directory should exist");

        // Verify it's a directory
        assert!(dir.is_dir(), "Session path should be a directory");
    }

    #[cfg(unix)]
    #[test]
    fn test_session_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let pid = std::process::id();
        let test_paths = vec!["test_path".to_string()];

        // Write to session file
        write_operation(pid, "test", &test_paths).unwrap();

        // Check file permissions
        let file_path = get_session_file(pid).unwrap();
        let metadata = fs::metadata(&file_path).unwrap();
        let permissions = metadata.permissions();

        // Should be 0600 (user read/write only)
        let mode = permissions.mode() & 0o777;
        assert_eq!(
            mode, 0o600,
            "Session file should have 0600 permissions, got {:o}",
            mode
        );

        // Cleanup
        let _ = clear_session(pid);
    }

    #[cfg(unix)]
    #[test]
    fn test_session_dir_permissions() {
        use std::os::unix::fs::PermissionsExt;

        // Get or create session directory
        let dir = get_session_dir().unwrap();

        // Check if directory exists and get its metadata
        assert!(dir.exists(), "Session directory should exist");
        let metadata = fs::metadata(&dir).unwrap();
        let permissions = metadata.permissions();

        // Should be 0700 (user rwx only)
        let mode = permissions.mode() & 0o777;
        assert_eq!(
            mode, 0o700,
            "Session directory should have 0700 permissions, got {:o}",
            mode
        );
    }

    #[test]
    fn test_write_and_read_session() {
        // Use a unique PID to avoid conflicts with other tests
        // Use a large number that's unlikely to be a real PID
        let pid = 999999;

        // Clear any existing session file first
        let _ = clear_session(pid);

        let test_paths = vec!["path1".to_string(), "path2".to_string()];

        // Write operations
        write_operation(pid, "moved", &test_paths).unwrap();
        write_operation(pid, "deleted", &["path3".to_string()]).unwrap();

        // Read back
        let (affected, deleted) = read_session_paths(pid).unwrap();

        assert!(
            affected.contains("path1"),
            "Expected 'path1' in affected paths, got: {:?}",
            affected
        );
        assert!(
            affected.contains("path2"),
            "Expected 'path2' in affected paths, got: {:?}",
            affected
        );
        assert_eq!(deleted, vec!["path3".to_string()]);

        // Cleanup
        let _ = clear_session(pid);
    }
}
