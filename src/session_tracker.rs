use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Get path to session log file for given PID
pub fn get_session_file(pid: u32) -> PathBuf {
    PathBuf::from(format!("/tmp/whi_session_{pid}.log"))
}

/// Read explicitly affected and deleted paths from the session log
pub fn read_session_paths(pid: u32) -> Result<(HashSet<String>, Vec<String>), String> {
    let session_file = get_session_file(pid);

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

    let session_file = get_session_file(pid);
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
    let session_file = get_session_file(pid);
    if session_file.exists() {
        fs::remove_file(&session_file).map_err(|e| format!("Failed to remove session log: {e}"))?;
    }
    Ok(())
}

/// Get all session files in /tmp
fn get_all_session_files() -> Result<Vec<(PathBuf, std::time::SystemTime)>, String> {
    let tmp_dir = Path::new("/tmp");
    let entries =
        fs::read_dir(tmp_dir).map_err(|e| format!("Failed to read /tmp directory: {e}"))?;

    let mut session_files = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name.starts_with("whi_session_") && name.ends_with(".log") {
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
pub fn cleanup_old_sessions() -> Result<(), String> {
    let mut session_files = get_all_session_files()?;

    if session_files.len() <= 30 {
        return Ok(());
    }

    // Sort by modification time (oldest first)
    session_files.sort_by(|a, b| a.1.cmp(&b.1));

    // Delete oldest files until we have 30 or fewer
    let files_to_delete = session_files.len() - 30;
    for (path, _) in session_files.iter().take(files_to_delete) {
        let _ = fs::remove_file(path); // Ignore errors
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_file_path() {
        let path = get_session_file(12345);
        assert_eq!(path, PathBuf::from("/tmp/whi_session_12345.log"));
    }
}
