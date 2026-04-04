mod support;

use std::fs;

use support::{env_lock, set_env_var};
use tempfile::TempDir;
use whi::session::history::HistoryContext;
use whi::session::store::{
    clear_cursor, clear_session, get_current_snapshot, get_initial_path, get_session_file,
    read_path_snapshots, set_cursor, truncate_snapshots, write_path_snapshot,
};

#[test]
fn history_write_and_read_snapshots() {
    let _lock = env_lock();
    let runtime_dir = TempDir::new().unwrap();
    let _runtime = set_env_var("XDG_RUNTIME_DIR", runtime_dir.path());

    let history = HistoryContext::global(10).unwrap();
    history.write_snapshot("/bin:/usr/bin").unwrap();
    history.write_snapshot("/usr/bin").unwrap();

    let snapshots = history.read_snapshots().unwrap();
    assert_eq!(snapshots, vec!["/bin:/usr/bin", "/usr/bin"]);
}

#[test]
fn history_cursor_operations() {
    let _lock = env_lock();
    let runtime_dir = TempDir::new().unwrap();
    let _runtime = set_env_var("XDG_RUNTIME_DIR", runtime_dir.path());

    let history = HistoryContext::global(11).unwrap();
    history.write_snapshot("/bin").unwrap();
    history.write_snapshot("/usr/bin").unwrap();

    history.set_cursor(0).unwrap();
    assert_eq!(history.get_cursor().unwrap(), Some(0));
    assert_eq!(
        history.current_snapshot().unwrap(),
        Some("/bin".to_string())
    );

    history.clear_cursor().unwrap();
    assert_eq!(history.get_cursor().unwrap(), None);
}

#[test]
fn store_session_file_path_and_directory_permissions() {
    let _lock = env_lock();
    let runtime_dir = TempDir::new().unwrap();
    let _runtime = set_env_var("XDG_RUNTIME_DIR", runtime_dir.path());

    let file_path = get_session_file(12).unwrap();
    assert!(file_path.to_string_lossy().contains("whi-"));
    assert!(file_path.to_string_lossy().contains("session_12.log"));

    write_path_snapshot(12, "/test/path").unwrap();

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let dir = file_path.parent().unwrap();
        let dir_mode = fs::metadata(dir).unwrap().permissions().mode() & 0o777;
        let file_mode = fs::metadata(&file_path).unwrap().permissions().mode() & 0o777;

        assert_eq!(dir_mode, 0o700);
        assert_eq!(file_mode, 0o600);
    }
}

#[test]
fn store_truncate_and_initial_path() {
    let _lock = env_lock();
    let runtime_dir = TempDir::new().unwrap();
    let _runtime = set_env_var("XDG_RUNTIME_DIR", runtime_dir.path());

    let pid = 13;
    write_path_snapshot(pid, "/initial").unwrap();
    write_path_snapshot(pid, "/snap1").unwrap();
    write_path_snapshot(pid, "/snap2").unwrap();

    assert_eq!(get_initial_path(pid).unwrap(), Some("/initial".to_string()));
    assert_eq!(
        get_current_snapshot(pid).unwrap(),
        Some("/snap2".to_string())
    );

    truncate_snapshots(pid, 2).unwrap();
    assert_eq!(
        read_path_snapshots(pid).unwrap(),
        vec!["/initial", "/snap1"]
    );

    set_cursor(pid, 0).unwrap();
    assert_eq!(
        get_current_snapshot(pid).unwrap(),
        Some("/initial".to_string())
    );
    clear_cursor(pid).unwrap();

    clear_session(pid).unwrap();
}
