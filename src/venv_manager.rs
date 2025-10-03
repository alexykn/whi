use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

use crate::atomic_file::AtomicFile;
use crate::history::HistoryContext;

const WHI_FILE: &str = "whi.file";
const WHI_LOCK: &str = "whi.lock";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenvType {
    File, // Editable
    Lock, // Read-only
}

#[derive(Debug, Clone)]
pub struct VenvTransition {
    pub new_path: String,
    pub set_vars: Vec<(String, String)>,
    pub unset_vars: Vec<String>,
}

/// Check if we're in a venv
pub fn is_in_venv() -> bool {
    env::var("WHI_VENV_NAME").is_ok()
}

/// Check if in locked venv
pub fn is_locked_venv() -> bool {
    env::var("WHI_VENV_LOCKED").as_deref() == Ok("1")
}

/// Get session PID from environment
fn get_session_pid() -> u32 {
    env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| std::process::id())
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
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to get user ID: {e}")))?;

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
fn get_venv_type_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("venv_type"))
}

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

/// Save venv info (type and directory)
fn save_venv_info(session_pid: u32, venv_type: VenvType, dir: &Path) -> io::Result<()> {
    let venv_type_file = get_venv_type_file(session_pid)?;
    let venv_dir_file = get_venv_dir_file(session_pid)?;

    fs::write(
        venv_type_file,
        match venv_type {
            VenvType::File => "file",
            VenvType::Lock => "lock",
        },
    )?;

    fs::write(venv_dir_file, dir.to_string_lossy().as_bytes())?;

    Ok(())
}

/// Clear venv info
fn clear_venv_info(session_pid: u32) -> io::Result<()> {
    if let Ok(restore_file) = get_venv_restore_file(session_pid) {
        let _ = fs::remove_file(restore_file);
    }
    if let Ok(type_file) = get_venv_type_file(session_pid) {
        let _ = fs::remove_file(type_file);
    }
    if let Ok(dir_file) = get_venv_dir_file(session_pid) {
        let _ = fs::remove_file(dir_file);
    }
    Ok(())
}

/// Create whi.file from current PATH
pub fn create_file(force: bool) -> io::Result<()> {
    use crate::path_file::format_path_file;

    let whi_file = Path::new(WHI_FILE);
    let whi_lock = Path::new(WHI_LOCK);

    // Check for whi.lock
    if whi_lock.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Project is locked. Run 'whi unlock' first",
        ));
    }

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
    println!("Saved PATH to ./whi.file ({} entries)", entries);

    Ok(())
}

/// Lock project: whi.file → whi.lock
pub fn lock() -> io::Result<()> {
    let whi_file = Path::new(WHI_FILE);
    let whi_lock = Path::new(WHI_LOCK);

    // Check if already locked first (before checking for whi.file)
    if whi_lock.exists() {
        println!("No changes made (already locked)");
        return Ok(());
    }

    if !whi_file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No whi.file found. Run 'whi file' first",
        ));
    }

    fs::rename(whi_file, whi_lock)?;
    println!("Locked ./whi.file → ./whi.lock");

    // If currently in venv, notify user
    if is_in_venv() && !is_locked_venv() {
        eprintln!("Exit and re-enter directory to activate locked mode.");
    }

    Ok(())
}

/// Unlock project: whi.lock → whi.file
pub fn unlock() -> io::Result<Option<VenvTransition>> {
    let whi_file = Path::new(WHI_FILE);
    let whi_lock = Path::new(WHI_LOCK);

    if !whi_lock.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "No whi.lock found"));
    }

    if whi_file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "whi.file already exists",
        ));
    }

    fs::rename(whi_lock, whi_file)?;

    // If we were inside a locked venv, update session info to editable mode
    if is_in_venv() && is_locked_venv() {
        let session_pid = get_session_pid();

        if let Ok(dir) = env::var("WHI_VENV_DIR") {
            let dir_path = Path::new(&dir);
            save_venv_info(session_pid, VenvType::File, dir_path)?;
        }

        let current_path = env::var("PATH").unwrap_or_default();
        let transition = VenvTransition {
            new_path: current_path,
            set_vars: vec![("WHI_VENV_LOCKED".to_string(), "0".to_string())],
            unset_vars: Vec::new(),
        };

        return Ok(Some(transition));
    }

    Ok(None)
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
    let whi_lock = dir.join(WHI_LOCK);

    // Determine which file to source (lock takes precedence)
    let (path_file, venv_type) = if whi_lock.exists() {
        (whi_lock, VenvType::Lock)
    } else if whi_file.exists() {
        (whi_file, VenvType::File)
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No whi.file or whi.lock found",
        ));
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
    let venv_name = dir
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "whi-venv".to_string());

    // Save current PATH for restore
    let session_pid = get_session_pid();
    let current_path = env::var("PATH").unwrap_or_default();
    save_venv_restore(session_pid, &current_path)?;
    save_venv_info(session_pid, venv_type, dir)?;

    HistoryContext::venv(session_pid, dir)
        .and_then(|ctx| ctx.reset_with_initial(&new_path))
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

    let set_vars = vec![
        ("WHI_VENV_NAME".to_string(), venv_name),
        (
            "WHI_VENV_LOCKED".to_string(),
            if venv_type == VenvType::Lock {
                "1".to_string()
            } else {
                "0".to_string()
            },
        ),
        ("WHI_VENV_DIR".to_string(), dir.display().to_string()),
    ];

    Ok(VenvTransition {
        new_path,
        set_vars,
        unset_vars: Vec::new(),
    })
}

/// Source venv from pwd (whi.file or whi.lock) - convenience wrapper
pub fn source() -> io::Result<VenvTransition> {
    let pwd = env::current_dir()?;
    source_from_path(&pwd.to_string_lossy())
}

/// Exit venv
pub fn exit_venv() -> io::Result<VenvTransition> {
    if !is_in_venv() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Not in a venv"));
    }

    // Check if locked venv requires cd exit (will be implemented with config)
    if is_locked_venv() && should_require_cd_exit()? {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot exit locked environment (leave directory instead)",
        ));
    }

    let session_pid = get_session_pid();
    let restored_path = restore_venv_path(session_pid)?;

    // Clear venv info
    clear_venv_info(session_pid)?;

    let unset_vars = vec![
        "WHI_VENV_NAME".to_string(),
        "WHI_VENV_LOCKED".to_string(),
        "WHI_LOCKED_DIR".to_string(),
        "WHI_VENV_DIR".to_string(),
    ];

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

/// Check if locked venv requires cd exit
fn should_require_cd_exit() -> io::Result<bool> {
    use crate::config::load_config;

    match load_config() {
        Ok(config) => Ok(config.venv.lock_require_cd_exit),
        Err(_) => Ok(false), // Default to false on error
    }
}

/// Check venv guard for modification operations
pub fn check_venv_modification_allowed() -> Result<(), String> {
    if is_locked_venv() {
        Err("Cannot modify PATH in locked environment".to_string())
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;
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
    fn test_lock_unlock() {
        let _guard = test_mutex().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        env::set_current_dir(&temp_path).unwrap();
        env::set_var("WHI_SESSION_PID", "12346");

        // Lock without file should fail
        assert!(lock().is_err());

        // Create file and lock
        env::set_var("PATH", "/usr/bin:/bin");
        create_file(false).unwrap();
        assert!(lock().is_ok());
        assert!(!temp_path.join(WHI_FILE).exists());
        assert!(temp_path.join(WHI_LOCK).exists());

        // Lock again should be no-op (but succeed)
        assert!(lock().is_ok());

        // Unlock
        env::remove_var("WHI_VENV_LOCKED"); // Not in venv
        let unlock_result = unlock().unwrap();
        assert!(unlock_result.is_none());
        assert!(temp_path.join(WHI_FILE).exists());
        assert!(!temp_path.join(WHI_LOCK).exists());

        // Cleanup
        env::remove_var("WHI_SESSION_PID");
    }

    #[test]
    fn test_create_file_with_lock_present() {
        let _guard = test_mutex().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        env::set_current_dir(&temp_path).unwrap();
        env::set_var("PATH", "/usr/bin:/bin");
        env::set_var("WHI_SESSION_PID", "12347");

        // Create whi.lock
        fs::write(temp_path.join(WHI_LOCK), "/usr/bin").unwrap();

        // Attempt to create whi.file should fail
        let result = create_file(false);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Project is locked"));

        // Cleanup
        env::remove_var("WHI_SESSION_PID");
    }

    #[test]
    fn test_check_venv_modification_allowed() {
        let _guard = test_mutex().lock().unwrap();
        // Not in locked venv
        env::remove_var("WHI_VENV_LOCKED");
        assert!(check_venv_modification_allowed().is_ok());

        // In locked venv
        env::set_var("WHI_VENV_LOCKED", "1");
        let result = check_venv_modification_allowed();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("locked environment"));

        // Cleanup
        env::remove_var("WHI_VENV_LOCKED");
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
    fn test_unlock_returns_transition_when_in_locked_env() {
        let _guard = test_mutex().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_current_dir(temp_dir.path()).unwrap();

        fs::write(WHI_LOCK, "PATH!\n/usr/bin\n\nENV!\n").unwrap();

        env::set_var("WHI_VENV_NAME", "test-venv");
        env::set_var("WHI_VENV_LOCKED", "1");
        env::set_var("WHI_VENV_DIR", temp_dir.path());
        env::set_var("WHI_SESSION_PID", "7777");
        env::set_var("PATH", "/foo:/bar");

        save_venv_restore(7777, "/restore:path").unwrap();

        let result = unlock().unwrap();
        assert!(result.is_some());
        let transition = result.unwrap();
        assert_eq!(transition.new_path, "/foo:/bar");
        assert_eq!(transition.set_vars.len(), 1);
        assert_eq!(transition.set_vars[0].0, "WHI_VENV_LOCKED");
        assert_eq!(transition.set_vars[0].1, "0");
        assert!(transition.unset_vars.is_empty());

        assert!(Path::new(WHI_FILE).exists());
        assert!(!Path::new(WHI_LOCK).exists());

        env::remove_var("WHI_VENV_NAME");
        env::remove_var("WHI_VENV_LOCKED");
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
