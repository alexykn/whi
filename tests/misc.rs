use std::collections::HashSet;
use std::path::PathBuf;

use whi::platform::{get_parent_pid, get_user_id, is_tty};

#[test]
fn platform_smoke_test() {
    let ppid = get_parent_pid().unwrap();
    let uid = get_user_id().unwrap();

    assert!(ppid > 0);
    assert!(uid < u32::MAX);

    let _ = is_tty(0);
    let _ = is_tty(1);
    let _ = is_tty(2);
}

#[test]
fn commands_runner_normalization() {
    let current = "/usr/local/sbin/:/usr/bin:/bin";
    let protected = [PathBuf::from("/usr/local/sbin")];

    let current_paths: HashSet<String> = current
        .split(':')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim_end_matches('/').to_string())
        .collect();

    let missing: Vec<String> = protected
        .iter()
        .filter_map(|p| {
            let path_str = p.to_string_lossy().to_string();
            let normalized = path_str.trim_end_matches('/');
            if current_paths.contains(normalized) {
                None
            } else {
                Some(path_str)
            }
        })
        .collect();

    assert!(missing.is_empty());
}
