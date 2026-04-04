use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::PathBuf;

#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

use crate::platform;
use crate::session::history::HistoryContext;

/// Get or create session directory (user-specific, secure)
fn get_session_dir() -> Result<PathBuf, String> {
    // Try XDG_RUNTIME_DIR first (standard for user-specific runtime files)
    let base_dir = env::var("XDG_RUNTIME_DIR")
        .or_else(|_| env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());

    // Use UID for additional security
    let uid = platform::get_user_id().map_err(|e| format!("Failed to get user ID: {e}"))?;

    let session_dir = PathBuf::from(format!("{base_dir}/whi-{uid}"));

    // Create directory with restrictive permissions (0700) if it doesn't exist
    if !session_dir.exists() {
        #[cfg(unix)]
        {
            if let Err(e) = fs::DirBuilder::new().mode(0o700).create(&session_dir)
                && e.kind() != ErrorKind::AlreadyExists
            {
                return Err(format!("Failed to create session dir: {e}"));
            }
        }

        #[cfg(not(unix))]
        {
            if let Err(e) = fs::create_dir_all(&session_dir) {
                if e.kind() != ErrorKind::AlreadyExists {
                    return Err(format!("Failed to create session dir: {}", e));
                }
            }
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
    HistoryContext::global(pid)?.write_snapshot(path_string)
}

/// Read all `PATH` snapshots from session log
pub fn read_path_snapshots(pid: u32) -> Result<Vec<String>, String> {
    HistoryContext::global(pid)?.read_snapshots()
}

/// Get the initial `PATH` snapshot (first snapshot in session)
pub fn get_initial_path(pid: u32) -> Result<Option<String>, String> {
    HistoryContext::global(pid)?.initial_snapshot()
}

/// Truncate snapshots to keep only the first `keep_count` snapshots
/// This is used by undo/reset to discard "future" snapshots from abandoned timelines
pub fn truncate_snapshots(pid: u32, keep_count: usize) -> Result<(), String> {
    HistoryContext::global(pid)?.truncate(keep_count)
}

/// Get cursor file path for given `PID`
/// Get current cursor position (index into snapshots)
/// Returns `None` if at end of history (no cursor file = at latest)
pub fn get_cursor(pid: u32) -> Result<Option<usize>, String> {
    HistoryContext::global(pid)?.get_cursor()
}

/// Set cursor position (index into snapshots)
pub fn set_cursor(pid: u32, position: usize) -> Result<(), String> {
    HistoryContext::global(pid)?.set_cursor(position)
}

/// Clear cursor (move to end of history)
pub fn clear_cursor(pid: u32) -> Result<(), String> {
    HistoryContext::global(pid)?.clear_cursor()
}

/// Get current `PATH` snapshot based on cursor position
pub fn get_current_snapshot(pid: u32) -> Result<Option<String>, String> {
    HistoryContext::global(pid)?.current_snapshot()
}

/// Clear the session log for given `PID`
pub fn clear_session(pid: u32) -> Result<(), String> {
    HistoryContext::global(pid)?.clear_history()
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
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.starts_with("session_")
            && path.extension().is_some_and(|ext| ext == "log")
            && let Ok(metadata) = entry.metadata()
            && let Ok(modified) = metadata.modified()
        {
            session_files.push((path, modified));
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
