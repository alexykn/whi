use std::fs;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::atomic_file::AtomicFile;
use crate::shell_detect::{get_config_file_path, get_saved_path_file, get_sourcing_line, Shell};

/// Save the current PATH for a shell
pub fn save_path(shell: &Shell, path: &str) -> Result<(), String> {
    let saved_path_file = get_saved_path_file(shell)?;

    // Create ~/.whi directory if it doesn't exist
    if let Some(parent) = saved_path_file.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create .whi directory: {e}"))?;
    }

    // Create backup if file exists
    if saved_path_file.exists() {
        let backup_path = saved_path_file.with_extension("bak");
        let mut atomic_backup = AtomicFile::new(&backup_path)
            .map_err(|e| format!("Failed to create backup file: {e}"))?;

        let existing_content = fs::read_to_string(&saved_path_file)
            .map_err(|e| format!("Failed to read existing PATH file: {e}"))?;

        atomic_backup
            .write_all(existing_content.as_bytes())
            .map_err(|e| format!("Failed to write backup: {e}"))?;

        atomic_backup
            .commit()
            .map_err(|e| format!("Failed to commit backup: {e}"))?;
    }

    // Write PATH atomically
    let mut atomic_file = AtomicFile::new(&saved_path_file)
        .map_err(|e| format!("Failed to create PATH file: {e}"))?;

    atomic_file
        .write_all(path.as_bytes())
        .map_err(|e| format!("Failed to write PATH: {e}"))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit PATH file: {e}"))?;

    // Ensure whi integration exists in config file
    ensure_whi_integration(shell)?;

    Ok(())
}

/// Load saved PATH for a shell
pub fn load_saved_path(shell: &Shell) -> Result<String, String> {
    let saved_path_file = get_saved_path_file(shell)?;

    if !saved_path_file.exists() {
        return Err(format!(
            "No saved PATH for {}. Run 'whi save {}' first.",
            shell.as_str(),
            shell.as_str()
        ));
    }

    fs::read_to_string(&saved_path_file).map_err(|e| format!("Failed to read saved PATH file: {e}"))
}

/// Ensure the whi integration line exists in the shell config file
pub fn ensure_whi_integration(shell: &Shell) -> Result<(), String> {
    let config_file = get_config_file_path(shell)?;
    let sourcing_line = get_sourcing_line(shell)?;

    // Read existing config (or empty if doesn't exist)
    let existing_content = if config_file.exists() {
        fs::read_to_string(&config_file).map_err(|e| format!("Failed to read config file: {e}"))?
    } else {
        String::new()
    };

    // Check if integration already exists
    if existing_content.contains("# whi: Load saved PATH") {
        return Ok(()); // Already integrated
    }

    // Create backup
    if config_file.exists() {
        backup_config_file(&config_file)?;
    } else {
        // Create parent directory for fish if needed
        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }
    }

    // Append integration line
    let new_content = if existing_content.is_empty() {
        sourcing_line
    } else if existing_content.ends_with('\n') {
        format!("{existing_content}\n{sourcing_line}")
    } else {
        format!("{existing_content}\n\n{sourcing_line}")
    };

    // Write atomically
    let mut atomic_file =
        AtomicFile::new(&config_file).map_err(|e| format!("Failed to create config file: {e}"))?;

    atomic_file
        .write_all(new_content.as_bytes())
        .map_err(|e| format!("Failed to write config: {e}"))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit config file: {e}"))?;

    Ok(())
}

/// Create a backup of a config file with timestamp
fn backup_config_file(path: &Path) -> Result<(), String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("Failed to get timestamp: {e}"))?
        .as_secs();

    let backup_path = path.with_extension(format!("backup-{timestamp}"));

    fs::copy(path, &backup_path).map_err(|e| format!("Failed to create backup: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;

    #[test]
    fn test_backup_creates_file() {
        let temp_dir = env::temp_dir().join(format!("whi-test-{}", timestamp_now()));
        fs::create_dir_all(&temp_dir).unwrap();

        let test_file = temp_dir.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        backup_config_file(&test_file).unwrap();

        // Check backup exists
        let backup_files: Vec<_> = fs::read_dir(&temp_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().starts_with("test.backup-"))
            .collect();

        assert_eq!(backup_files.len(), 1);

        // Cleanup
        fs::remove_dir_all(temp_dir).ok();
    }

    fn timestamp_now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
    }
}
