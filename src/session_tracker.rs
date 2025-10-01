use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

use crate::system;

/// Maximum snapshots to keep per session (keeps initial + last 499)
const MAX_SNAPSHOTS_PER_SESSION: usize = 500;

/// Get or create session directory (user-specific, secure)
fn get_session_dir() -> Result<PathBuf, String> {
    // Try XDG_RUNTIME_DIR first (standard for user-specific runtime files)
    let base_dir = env::var("XDG_RUNTIME_DIR")
        .or_else(|_| env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());

    // Use UID for additional security
    let uid = system::get_user_id().map_err(|e| format!("Failed to get user ID: {e}"))?;

    let session_dir = PathBuf::from(format!("{base_dir}/whi-{uid}"));

    // Create directory with restrictive permissions (0700) if it doesn't exist
    if !session_dir.exists() {
        #[cfg(unix)]
        {
            fs::DirBuilder::new()
                .mode(0o700)
                .create(&session_dir)
                .map_err(|e| format!("Failed to create session dir: {e}"))?;
        }

        #[cfg(not(unix))]
        {
            fs::create_dir_all(&session_dir)
                .map_err(|e| format!("Failed to create session dir: {}", e))?;
        }
    }

    Ok(session_dir)
}

/// Get path to session log file for given `PID`
pub fn get_session_file(pid: u32) -> Result<PathBuf, String> {
    let session_dir = get_session_dir()?;
    Ok(session_dir.join(format!("session_{pid}.log")))
}

/// Write `PATH` snapshot to session log
pub fn write_path_snapshot(pid: u32, path_string: &str) -> Result<(), String> {
    use std::time::{SystemTime, UNIX_EPOCH};

    // Check if we have a cursor (not at end of history)
    // If so, truncate after cursor to create new timeline
    if let Some(cursor) = get_cursor(pid)? {
        truncate_snapshots(pid, cursor + 1)?;
    }

    let session_file = get_session_file(pid)?;

    #[cfg(unix)]
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log: {e}"))?;

    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log: {e}"))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to get timestamp: {e}"))?
        .as_secs();

    // Format: SNAPSHOT:timestamp:path
    writeln!(file, "SNAPSHOT:{timestamp}:{path_string}")
        .map_err(|e| format!("Failed to write PATH snapshot: {e}"))?;

    // Drop file handle before potential operations
    drop(file);

    // Clear cursor (we're back at end of history)
    clear_cursor(pid)?;

    // Check if we need to cleanup old snapshots (keep initial + last N-1)
    let snapshots = read_path_snapshots(pid)?;
    if snapshots.len() > MAX_SNAPSHOTS_PER_SESSION {
        // Keep snapshot 0 (initial) + last 24 snapshots
        // This allows 'reset' to always work and 'undo' to work for ~24 operations
        truncate_to_keep_initial_and_tail(pid, MAX_SNAPSHOTS_PER_SESSION)?;
    }

    Ok(())
}

/// Read all `PATH` snapshots from session log
pub fn read_path_snapshots(pid: u32) -> Result<Vec<String>, String> {
    let session_file = get_session_file(pid)?;

    if !session_file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&session_file)
        .map_err(|e| format!("Failed to read session log: {e}"))?;

    let mut snapshots = Vec::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("SNAPSHOT:") {
            // Format: SNAPSHOT:timestamp:path
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() >= 2 {
                // parts[0] = timestamp, parts[1] = path
                snapshots.push(parts[1].to_string());
            }
        }
    }

    Ok(snapshots)
}

/// Get the initial `PATH` snapshot (first snapshot in session)
pub fn get_initial_path(pid: u32) -> Result<Option<String>, String> {
    let snapshots = read_path_snapshots(pid)?;
    Ok(snapshots.first().cloned())
}

/// Truncate snapshots to keep only the first `keep_count` snapshots
/// This is used by undo/reset to discard "future" snapshots from abandoned timelines
pub fn truncate_snapshots(pid: u32, keep_count: usize) -> Result<(), String> {
    let session_file = get_session_file(pid)?;

    if !session_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&session_file)
        .map_err(|e| format!("Failed to read session log: {e}"))?;

    // Keep only the first keep_count snapshot lines
    let mut new_lines = Vec::new();
    let mut snapshot_count = 0;

    for line in content.lines() {
        if line.starts_with("SNAPSHOT:") && snapshot_count < keep_count {
            new_lines.push(line.to_string());
            snapshot_count += 1;
        }
    }

    #[cfg(unix)]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log for truncation: {e}"))?;

    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log for truncation: {e}"))?;

    for line in new_lines {
        writeln!(file, "{line}")
            .map_err(|e| format!("Failed to write truncated session log: {e}"))?;
    }

    Ok(())
}

/// Truncate snapshots to keep initial + last N-1 snapshots (rolling window)
/// This is used for within-session cleanup to prevent unbounded growth
fn truncate_to_keep_initial_and_tail(pid: u32, max_snapshots: usize) -> Result<(), String> {
    let session_file = get_session_file(pid)?;

    if !session_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&session_file)
        .map_err(|e| format!("Failed to read session log: {e}"))?;

    // Count total snapshots
    let total_snapshots = content
        .lines()
        .filter(|l| l.starts_with("SNAPSHOT:"))
        .count();

    if total_snapshots <= max_snapshots {
        return Ok(()); // No cleanup needed
    }

    // Calculate which snapshots to keep:
    // - Always keep snapshot 0 (initial)
    // - Keep last (max_snapshots - 1) snapshots
    let drop_count = total_snapshots - max_snapshots;

    // Keep snapshots at indices: [0] + [drop_count+1, drop_count+2, ..., total-1]
    let mut new_lines = Vec::new();
    let mut snapshot_index = 0;

    for line in content.lines() {
        if line.starts_with("SNAPSHOT:") {
            // Keep snapshot 0 or snapshots after drop window
            if snapshot_index == 0 || snapshot_index > drop_count {
                new_lines.push(line.to_string());
            }
            snapshot_index += 1;
        }
    }

    #[cfg(unix)]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log for truncation: {e}"))?;

    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&session_file)
        .map_err(|e| format!("Failed to open session log for truncation: {e}"))?;

    for line in new_lines {
        writeln!(file, "{line}")
            .map_err(|e| format!("Failed to write truncated session log: {e}"))?;
    }

    Ok(())
}

/// Get cursor file path for given `PID`
fn get_cursor_file(pid: u32) -> Result<PathBuf, String> {
    let session_dir = get_session_dir()?;
    Ok(session_dir.join(format!("session_{pid}.cursor")))
}

/// Get current cursor position (index into snapshots)
/// Returns `None` if at end of history (no cursor file = at latest)
pub fn get_cursor(pid: u32) -> Result<Option<usize>, String> {
    let cursor_file = get_cursor_file(pid)?;

    if !cursor_file.exists() {
        return Ok(None); // At end of history
    }

    let content =
        fs::read_to_string(&cursor_file).map_err(|e| format!("Failed to read cursor file: {e}"))?;

    content
        .trim()
        .parse::<usize>()
        .map(Some)
        .map_err(|e| format!("Invalid cursor value: {e}"))
}

/// Set cursor position (index into snapshots)
pub fn set_cursor(pid: u32, position: usize) -> Result<(), String> {
    let cursor_file = get_cursor_file(pid)?;

    fs::write(&cursor_file, position.to_string())
        .map_err(|e| format!("Failed to write cursor file: {e}"))
}

/// Clear cursor (move to end of history)
pub fn clear_cursor(pid: u32) -> Result<(), String> {
    let cursor_file = get_cursor_file(pid)?;

    if cursor_file.exists() {
        fs::remove_file(&cursor_file).map_err(|e| format!("Failed to remove cursor file: {e}"))?;
    }

    Ok(())
}

/// Get current `PATH` snapshot based on cursor position
pub fn get_current_snapshot(pid: u32) -> Result<Option<String>, String> {
    let snapshots = read_path_snapshots(pid)?;

    if snapshots.is_empty() {
        return Ok(None);
    }

    let cursor = get_cursor(pid)?.unwrap_or(snapshots.len() - 1);

    if cursor >= snapshots.len() {
        return Err(format!(
            "Cursor position {cursor} exceeds history length {}",
            snapshots.len()
        ));
    }

    Ok(Some(snapshots[cursor].clone()))
}

/// Clear the session log for given `PID`
pub fn clear_session(pid: u32) -> Result<(), String> {
    let session_file = get_session_file(pid)?;
    if session_file.exists() {
        fs::remove_file(&session_file).map_err(|e| format!("Failed to remove session log: {e}"))?;
    }

    // Also clear cursor
    clear_cursor(pid)?;

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
            if name.starts_with("session_") && path.extension().is_some_and(|ext| ext == "log") {
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
            path_str.contains("whi-")
                && path.extension().is_some_and(|ext| ext == "log")
                && path_str.contains("session_12345"),
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

        let pid = 999001;
        let _ = clear_session(pid);

        // Write to session file
        write_path_snapshot(pid, "/test/path").unwrap();

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
    fn test_write_and_read_snapshots() {
        let pid = 999002;
        let _ = clear_session(pid);

        // Write snapshots
        write_path_snapshot(pid, "/a:/b:/c").unwrap();
        write_path_snapshot(pid, "/b:/c:/a").unwrap();
        write_path_snapshot(pid, "/c:/a:/b").unwrap();

        // Read back
        let snapshots = read_path_snapshots(pid).unwrap();

        assert_eq!(snapshots.len(), 3);
        assert_eq!(snapshots[0], "/a:/b:/c");
        assert_eq!(snapshots[1], "/b:/c:/a");
        assert_eq!(snapshots[2], "/c:/a:/b");

        // Cleanup
        let _ = clear_session(pid);
    }

    #[test]
    fn test_snapshot_truncation() {
        let pid = 999003;
        let _ = clear_session(pid);

        // Write 5 snapshots
        write_path_snapshot(pid, "/initial").unwrap();
        write_path_snapshot(pid, "/snap1").unwrap();
        write_path_snapshot(pid, "/snap2").unwrap();
        write_path_snapshot(pid, "/snap3").unwrap();
        write_path_snapshot(pid, "/snap4").unwrap();

        // Verify all 5
        let snapshots = read_path_snapshots(pid).unwrap();
        assert_eq!(snapshots.len(), 5);

        // Truncate to keep only first 2
        truncate_snapshots(pid, 2).unwrap();

        // Verify only 2 remain
        let snapshots = read_path_snapshots(pid).unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0], "/initial");
        assert_eq!(snapshots[1], "/snap1");

        // Cleanup
        let _ = clear_session(pid);
    }

    #[test]
    fn test_rolling_window_cleanup() {
        let pid = 999004;
        let _ = clear_session(pid);

        // Write initial snapshot
        write_path_snapshot(pid, "/initial").unwrap();

        // Write 10 more snapshots
        for i in 1..=10 {
            write_path_snapshot(pid, &format!("/snapshot{}", i)).unwrap();
        }

        let snapshots = read_path_snapshots(pid).unwrap();
        assert_eq!(snapshots.len(), 11);

        // Manually call cleanup with max=5
        // Should keep snapshot 0 + last 4 (indices 7, 8, 9, 10)
        truncate_to_keep_initial_and_tail(pid, 5).unwrap();

        let snapshots = read_path_snapshots(pid).unwrap();
        assert_eq!(snapshots.len(), 5);
        assert_eq!(snapshots[0], "/initial");
        assert_eq!(snapshots[1], "/snapshot7");
        assert_eq!(snapshots[2], "/snapshot8");
        assert_eq!(snapshots[3], "/snapshot9");
        assert_eq!(snapshots[4], "/snapshot10");

        // Cleanup
        let _ = clear_session(pid);
    }

    #[test]
    fn test_get_initial_path() {
        let pid = 999005;
        let _ = clear_session(pid);

        // No snapshots yet
        assert_eq!(get_initial_path(pid).unwrap(), None);

        // Write snapshots
        write_path_snapshot(pid, "/first").unwrap();
        write_path_snapshot(pid, "/second").unwrap();

        // Get initial should return first
        assert_eq!(get_initial_path(pid).unwrap(), Some("/first".to_string()));

        // Cleanup
        let _ = clear_session(pid);
    }

    #[test]
    fn test_duplicate_init_handled() {
        let pid = 999006;
        let _ = clear_session(pid);

        // Simulate double-source scenario (user sources integration script twice)
        // This should be prevented by shell integration guards, but test resilience
        write_path_snapshot(pid, "/usr/bin:/bin").unwrap();
        write_path_snapshot(pid, "/usr/bin:/bin").unwrap(); // Duplicate init

        // Initial should still return first snapshot
        assert_eq!(
            get_initial_path(pid).unwrap(),
            Some("/usr/bin:/bin".to_string())
        );

        // Verify we can read both snapshots (not ideal, but handled)
        let snapshots = read_path_snapshots(pid).unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0], "/usr/bin:/bin");
        assert_eq!(snapshots[1], "/usr/bin:/bin");

        // Verify undo behavior - should go back to first duplicate, then error
        // (This demonstrates the phantom operation issue the guard prevents)
        assert_eq!(snapshots.len() - 1, 1); // Only 1 operation to undo

        // Cleanup
        let _ = clear_session(pid);
    }
}
