use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

use crate::atomic_file::AtomicFile;
use crate::history::HistoryContext;

const WHI_FILE: &str = "whi.file";

#[derive(Debug, Clone)]
pub struct VenvTransition {
    pub new_path: String,
    pub set_vars: Vec<(String, String)>,
    pub unset_vars: Vec<String>,
}

/// Check if we're in a venv
#[must_use]
pub fn is_in_venv() -> bool {
    env::var("WHI_VENV_NAME").is_ok()
}

/// Get session PID from environment
fn get_session_pid() -> u32 {
    env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id)
}

/// Get session directory for this session
fn get_session_dir(session_pid: u32) -> io::Result<PathBuf> {
    use crate::system;
    use std::env;

    // Try XDG_RUNTIME_DIR first (standard for user-specific runtime files)
    let base_dir = env::var("XDG_RUNTIME_DIR")
        .or_else(|_| env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());

    // Use UID for additional security
    let uid = system::get_user_id()
        .map_err(|e| io::Error::other(format!("Failed to get user ID: {e}")))?;

    let session_dir = PathBuf::from(format!("{base_dir}/whi-{uid}/session_{session_pid}"));

    // Create directory if it doesn't exist
    if !session_dir.exists() {
        #[cfg(unix)]
        {
            std::fs::DirBuilder::new()
                .mode(0o700)
                .recursive(true)
                .create(&session_dir)?;
        }

        #[cfg(not(unix))]
        {
            fs::create_dir_all(&session_dir)?;
        }
    }

    Ok(session_dir)
}

/// Get venv restore file path
fn get_venv_restore_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("venv_restore"))
}

/// Get venv type file path
/// Get venv dir file path
fn get_venv_dir_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("venv_dir"))
}

/// Save PATH for venv restore
fn save_venv_restore(session_pid: u32, path: &str) -> io::Result<()> {
    let restore_file = get_venv_restore_file(session_pid)?;
    fs::write(restore_file, path)?;
    Ok(())
}

/// Restore venv PATH
fn restore_venv_path(session_pid: u32) -> io::Result<String> {
    let restore_file = get_venv_restore_file(session_pid)?;
    let path = fs::read_to_string(restore_file)?;
    Ok(path.trim().to_string())
}

/// Save venv info (directory)
fn save_venv_info(session_pid: u32, dir: &Path) -> io::Result<()> {
    let venv_dir_file = get_venv_dir_file(session_pid)?;
    fs::write(venv_dir_file, dir.to_string_lossy().as_bytes())?;

    Ok(())
}

/// Clear venv info
fn clear_venv_info(session_pid: u32) {
    if let Ok(restore_file) = get_venv_restore_file(session_pid) {
        let _ = fs::remove_file(restore_file);
    }
    if let Ok(dir_file) = get_venv_dir_file(session_pid) {
        let _ = fs::remove_file(dir_file);
    }
}

/// Create whi.file from current PATH
pub fn create_file(force: bool) -> io::Result<()> {
    use crate::path_file::format_path_file;

    let whi_file = Path::new(WHI_FILE);

    // Check for existing whi.file
    if whi_file.exists() && !force {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "whi.file already exists. Use -f/--force to replace it",
        ));
    }

    // Get current PATH
    let path_var = env::var("PATH").unwrap_or_default();

    // Format as human-friendly file
    let formatted = format_path_file(&path_var);

    // Write atomically
    let mut atomic_file = AtomicFile::new(whi_file)?;
    atomic_file.write_all(formatted.as_bytes())?;
    atomic_file.commit()?;

    let entries = path_var.split(':').filter(|s| !s.is_empty()).count();
    println!("Saved PATH to ./whi.file ({entries} entries)");

    Ok(())
}

/// Source venv from specific path (used by shell integration)
pub fn source_from_path(dir_path: &str) -> io::Result<VenvTransition> {
    use crate::path_file::parse_path_file;

    // Check if already in venv
    if is_in_venv() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Already in a venv. Run 'whi exit' first",
        ));
    }

    let dir = Path::new(dir_path);
    let whi_file = dir.join(WHI_FILE);

    let path_file = if whi_file.exists() {
        whi_file
    } else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "No whi.file found"));
    };

    // Read and parse PATH from file
    let file_content = fs::read_to_string(&path_file)?;
    let new_path = parse_path_file(&file_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {}: {}", path_file.display(), e),
        )
    })?;

    // Get directory name for venv name
    let venv_name = dir.file_name().map_or_else(
        || "whi-venv".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );

    // Save current PATH for restore
    let session_pid = get_session_pid();
    let current_path = env::var("PATH").unwrap_or_default();
    save_venv_restore(session_pid, &current_path)?;
    save_venv_info(session_pid, dir)?;

    HistoryContext::venv(session_pid, dir)
        .and_then(|ctx| ctx.reset_with_initial(&new_path))
        .map_err(io::Error::other)?;

    let set_vars = vec![
        ("WHI_VENV_NAME".to_string(), venv_name),
        ("WHI_VENV_DIR".to_string(), dir.display().to_string()),
    ];

    Ok(VenvTransition {
        new_path,
        set_vars,
        unset_vars: Vec::new(),
    })
}

/// Source venv from pwd (whi.file) - convenience wrapper
pub fn source() -> io::Result<VenvTransition> {
    let pwd = env::current_dir()?;
    source_from_path(&pwd.to_string_lossy())
}

/// Exit venv
pub fn exit_venv() -> io::Result<VenvTransition> {
    if !is_in_venv() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Not in a venv"));
    }

    let session_pid = get_session_pid();
    let restored_path = restore_venv_path(session_pid)?;

    // Clear venv info
    clear_venv_info(session_pid);

    let unset_vars = vec!["WHI_VENV_NAME".to_string(), "WHI_VENV_DIR".to_string()];

    Ok(VenvTransition {
        new_path: restored_path,
        set_vars: Vec::new(),
        unset_vars,
    })
}

/// Update the stored restore PATH for the active venv
pub fn update_restore_path(new_path: &str) -> io::Result<()> {
    if !is_in_venv() {
        return Ok(());
    }

    let session_pid = get_session_pid();
    save_venv_restore(session_pid, new_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::sync::{Mutex, OnceLock};
    use tempfile::TempDir;

    fn test_mutex() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_create_file() {
        let _guard = test_mutex().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        env::set_current_dir(&temp_path).unwrap();
        env::set_var("PATH", "/usr/bin:/bin");
        env::set_var("WHI_SESSION_PID", "12345");

        // First create should succeed
        assert!(create_file(false).is_ok());
        assert!(temp_path.join(WHI_FILE).exists());

        // Second create without --force should fail
        let result = create_file(false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        // With --force should succeed
        assert!(create_file(true).is_ok());

        // Cleanup
        env::remove_var("WHI_SESSION_PID");
    }

    #[test]
    fn test_update_restore_path_refreshes_backup() {
        let _guard = test_mutex().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_var("WHI_VENV_NAME", "test-venv");
        env::set_var("WHI_SESSION_PID", "4242");

        save_venv_restore(4242, "/old:path").unwrap();
        update_restore_path("/new:path").unwrap();
        let restored = restore_venv_path(4242).unwrap();
        assert_eq!(restored, "/new:path");

        env::remove_var("WHI_VENV_NAME");
        env::remove_var("WHI_SESSION_PID");
        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_source_from_path_reads_whi_file() {
        let _guard = test_mutex().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_current_dir(temp_dir.path()).unwrap();
        env::set_var("WHI_SESSION_PID", "7777");
        env::set_var("PATH", "/usr/bin:/bin");
        env::remove_var("WHI_VENV_NAME");

        fs::write(WHI_FILE, "PATH!\n/usr/bin\n/bin\n\nENV!\n").unwrap();

        let transition = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();
        assert_eq!(transition.new_path, "/usr/bin:/bin");

        env::remove_var("WHI_VENV_NAME");
        env::remove_var("WHI_VENV_DIR");
        env::remove_var("WHI_SESSION_PID");
        env::remove_var("PATH");

        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }
}
