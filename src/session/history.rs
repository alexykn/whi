use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};

use crate::session::store;

/// Maximum history snapshots to keep (matches session tracker behaviour)
pub const MAX_HISTORY_SNAPSHOTS: usize = 500;

#[derive(Debug, Clone)]
pub struct HistoryFiles {
    pub history_file: PathBuf,
    pub cursor_file: PathBuf,
}

#[derive(Debug, Clone)]
pub struct HistoryContext {
    files: HistoryFiles,
}

impl HistoryContext {
    pub fn global(pid: u32) -> Result<Self, String> {
        Ok(Self {
            files: global_history_files(pid)?,
        })
    }

    pub fn write_snapshot(&self, path: &str) -> Result<(), String> {
        write_snapshot(&self.files, path, MAX_HISTORY_SNAPSHOTS)
    }

    pub fn reset_with_initial(&self, path: &str) -> Result<(), String> {
        clear_history(&self.files)?;
        self.write_snapshot(path)
    }

    pub fn read_snapshots(&self) -> Result<Vec<String>, String> {
        read_snapshots(&self.files)
    }

    pub fn initial_snapshot(&self) -> Result<Option<String>, String> {
        Ok(self.read_snapshots()?.into_iter().next())
    }

    pub fn truncate(&self, keep_count: usize) -> Result<(), String> {
        truncate_snapshots(&self.files, keep_count)
    }

    pub fn truncate_keep_initial_and_tail(&self, max_snapshots: usize) -> Result<(), String> {
        truncate_to_keep_initial_and_tail(&self.files, max_snapshots)
    }

    pub fn get_cursor(&self) -> Result<Option<usize>, String> {
        get_cursor(&self.files)
    }

    pub fn set_cursor(&self, position: usize) -> Result<(), String> {
        set_cursor(&self.files, position)
    }

    pub fn clear_cursor(&self) -> Result<(), String> {
        clear_cursor(&self.files)
    }

    pub fn current_snapshot(&self) -> Result<Option<String>, String> {
        current_snapshot(&self.files)
    }

    pub fn clear_history(&self) -> Result<(), String> {
        clear_history(&self.files)
    }
}

fn global_history_files(pid: u32) -> Result<HistoryFiles, String> {
    let history_file = store::get_session_file(pid)?;
    let session_dir = history_file
        .parent()
        .ok_or_else(|| "Failed to determine session directory".to_string())?
        .to_path_buf();
    let cursor_file = session_dir.join(format!("session_{pid}.cursor"));

    Ok(HistoryFiles {
        history_file,
        cursor_file,
    })
}

fn create_dir_if_missing(path: &Path) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }

    #[cfg(unix)]
    {
        if let Err(e) = fs::DirBuilder::new()
            .mode(0o700)
            .recursive(true)
            .create(path)
        {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(format!(
                    "Failed to create directory {}: {e}",
                    path.display()
                ));
            }
        }
    }

    #[cfg(not(unix))]
    {
        if let Err(e) = fs::create_dir_all(path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(format!(
                    "Failed to create directory {}: {e}",
                    path.display()
                ));
            }
        }
    }

    Ok(())
}

fn write_snapshot(
    files: &HistoryFiles,
    path_string: &str,
    max_snapshots: usize,
) -> Result<(), String> {
    if let Some(cursor) = get_cursor(files)? {
        truncate_snapshots(files, cursor + 1)?;
    }

    #[cfg(unix)]
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&files.history_file)
        .map_err(|e| format!("Failed to open history file: {e}"))?;

    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&files.history_file)
        .map_err(|e| format!("Failed to open history file: {e}"))?;

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to get timestamp: {e}"))?
        .as_secs();

    writeln!(file, "SNAPSHOT:{timestamp}:{path_string}")
        .map_err(|e| format!("Failed to write history snapshot: {e}"))?;

    drop(file);

    clear_cursor(files)?;

    truncate_to_keep_initial_and_tail(files, max_snapshots)?;

    Ok(())
}

fn read_snapshots(files: &HistoryFiles) -> Result<Vec<String>, String> {
    if !files.history_file.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&files.history_file)
        .map_err(|e| format!("Failed to read history file: {e}"))?;

    let mut snapshots = Vec::new();

    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("SNAPSHOT:") {
            let parts: Vec<&str> = rest.splitn(2, ':').collect();
            if parts.len() >= 2 {
                snapshots.push(parts[1].to_string());
            }
        }
    }

    Ok(snapshots)
}

fn truncate_snapshots(files: &HistoryFiles, keep_count: usize) -> Result<(), String> {
    if !files.history_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&files.history_file)
        .map_err(|e| format!("Failed to read history file: {e}"))?;

    let mut new_lines = Vec::new();
    let mut snapshot_count = 0;

    for line in content.lines() {
        if line.starts_with("SNAPSHOT:") {
            if snapshot_count < keep_count {
                new_lines.push(line.to_string());
            }
            snapshot_count += 1;
        }
    }

    #[cfg(unix)]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&files.history_file)
        .map_err(|e| format!("Failed to open history file for truncation: {e}"))?;

    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&files.history_file)
        .map_err(|e| format!("Failed to open history file for truncation: {e}"))?;

    for line in new_lines {
        writeln!(file, "{line}").map_err(|e| format!("Failed to write history file: {e}"))?;
    }

    Ok(())
}

fn truncate_to_keep_initial_and_tail(
    files: &HistoryFiles,
    max_snapshots: usize,
) -> Result<(), String> {
    if !files.history_file.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&files.history_file)
        .map_err(|e| format!("Failed to read history file: {e}"))?;

    let total_snapshots = content
        .lines()
        .filter(|l| l.starts_with("SNAPSHOT:"))
        .count();

    if total_snapshots <= max_snapshots {
        return Ok(());
    }

    let drop_count = total_snapshots - max_snapshots;

    let mut new_lines = Vec::new();
    let mut snapshot_index = 0;

    for line in content.lines() {
        if line.starts_with("SNAPSHOT:") {
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
        .open(&files.history_file)
        .map_err(|e| format!("Failed to open history file for truncation: {e}"))?;

    #[cfg(not(unix))]
    let mut file = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&files.history_file)
        .map_err(|e| format!("Failed to open history file for truncation: {e}"))?;

    for line in new_lines {
        writeln!(file, "{line}").map_err(|e| format!("Failed to write history file: {e}"))?;
    }

    Ok(())
}

fn get_cursor(files: &HistoryFiles) -> Result<Option<usize>, String> {
    if !files.cursor_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&files.cursor_file)
        .map_err(|e| format!("Failed to read cursor file: {e}"))?;

    content
        .trim()
        .parse::<usize>()
        .map(Some)
        .map_err(|e| format!("Invalid cursor value: {e}"))
}

fn set_cursor(files: &HistoryFiles, position: usize) -> Result<(), String> {
    if let Some(parent) = files.cursor_file.parent() {
        create_dir_if_missing(parent)?;
    }
    fs::write(&files.cursor_file, position.to_string())
        .map_err(|e| format!("Failed to write cursor file: {e}"))
}

fn clear_cursor(files: &HistoryFiles) -> Result<(), String> {
    if files.cursor_file.exists() {
        fs::remove_file(&files.cursor_file)
            .map_err(|e| format!("Failed to remove cursor file: {e}"))?;
    }
    Ok(())
}

fn current_snapshot(files: &HistoryFiles) -> Result<Option<String>, String> {
    let snapshots = read_snapshots(files)?;

    if snapshots.is_empty() {
        return Ok(None);
    }

    let cursor = get_cursor(files)?.unwrap_or(snapshots.len() - 1);

    if cursor >= snapshots.len() {
        return Err(format!(
            "Cursor position {cursor} exceeds history length {}",
            snapshots.len()
        ));
    }

    Ok(Some(snapshots[cursor].clone()))
}

fn clear_history(files: &HistoryFiles) -> Result<(), String> {
    if files.history_file.exists() {
        fs::remove_file(&files.history_file)
            .map_err(|e| format!("Failed to remove history file: {e}"))?;
    }
    clear_cursor(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_and_read_snapshots() {
        let dir = TempDir::new().unwrap();
        let files = HistoryFiles {
            history_file: dir.path().join("history.log"),
            cursor_file: dir.path().join("cursor"),
        };

        write_snapshot(&files, "/bin:/usr/bin", 10).unwrap();
        write_snapshot(&files, "/usr/bin", 10).unwrap();

        let snapshots = read_snapshots(&files).unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0], "/bin:/usr/bin");
    }

    #[test]
    fn cursor_operations() {
        let dir = TempDir::new().unwrap();
        let files = HistoryFiles {
            history_file: dir.path().join("history.log"),
            cursor_file: dir.path().join("cursor"),
        };

        write_snapshot(&files, "/bin", 10).unwrap();
        write_snapshot(&files, "/usr/bin", 10).unwrap();

        set_cursor(&files, 0).unwrap();
        assert_eq!(get_cursor(&files).unwrap(), Some(0));

        let current = current_snapshot(&files).unwrap().unwrap();
        assert_eq!(current, "/bin");

        clear_cursor(&files).unwrap();
        assert_eq!(get_cursor(&files).unwrap(), None);
    }
}
