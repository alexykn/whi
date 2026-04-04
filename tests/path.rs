mod support;

use std::path::Path;

use support::{env_lock, set_env_var};
use whi::path::PathSearcher;
use whi::path::diff::{DiffEntry, compute_diff};
use whi::path::file::{apply_path_sections, expand_shell_vars, format_path_file, parse_path_file};
use whi::path::fuzzy::FuzzyMatcher;
use whi::path::guard::PathGuard;
use whi::path::resolve::expand_tilde;

#[test]
fn expand_tilde_and_fuzzy_matcher() {
    let _lock = env_lock();
    let _home = set_env_var("HOME", "/home/testuser");

    assert_eq!(expand_tilde("~/bin"), "/home/testuser/bin");
    assert_eq!(expand_tilde("~"), "/home/testuser");

    let matcher = FuzzyMatcher::new("cargo bin");
    assert!(matcher.matches(Path::new("/Users/alxknt/.cargo/bin")));
    assert!(!matcher.matches(Path::new("/usr/local/bin")));
}

#[test]
fn path_file_roundtrip_and_shell_vars() {
    let _lock = env_lock();
    let _home = set_env_var("HOME", "/home/testuser");

    let original = "/usr/bin:/bin:/usr/local/bin:/opt/bin";
    let parsed = parse_path_file(&format_path_file(original)).unwrap();
    let reconstructed = apply_path_sections("", &parsed.path).unwrap();
    assert_eq!(reconstructed, original);

    assert_eq!(
        expand_shell_vars("~/bin:$HOME/tools:${HOME}/more"),
        "/home/testuser/bin:/home/testuser/tools:/home/testuser/more"
    );
}

#[test]
fn path_file_validation_error() {
    let result = parse_path_file("!path.replace\n/usr/bin\n\n!path.prepend\n/opt/bin\n");
    assert!(result.unwrap_err().contains("Cannot combine"));
}

#[test]
fn path_searcher_mutations() {
    let searcher = PathSearcher::new("/a:/b:/c:/d:/e");

    assert_eq!(searcher.move_entry(5, 2).unwrap(), "/a:/e:/b:/c:/d");
    assert_eq!(searcher.swap_entries(2, 4).unwrap(), "/a:/d:/c:/b:/e");
    assert_eq!(searcher.delete_entries(&[2, 4]).unwrap(), "/a:/c:/e");

    let (cleaned, removed) = searcher.clean_duplicates();
    assert_eq!(cleaned, "/a:/b:/c:/d:/e");
    assert!(removed.is_empty());
}

#[test]
fn path_searcher_security_filters() {
    let searcher = PathSearcher::new("/good:/bad\0path:/also_good");
    let dirs = searcher.dirs();

    assert_eq!(dirs.len(), 2);
    assert_eq!(dirs[0].to_str().unwrap(), "/good");
    assert_eq!(dirs[1].to_str().unwrap(), "/also_good");
}

#[test]
fn path_diff_mixed_changes() {
    let diff = compute_diff("/d:/a:/c", "/a:/b:/c", false);

    assert!(
        diff.entries
            .iter()
            .any(|e| matches!(e, DiffEntry::Removed(p) if p == "/b"))
    );
    assert!(
        diff.entries
            .iter()
            .any(|e| matches!(e, DiffEntry::Added(p) if p == "/d"))
    );
    assert!(
        diff.entries
            .iter()
            .any(|e| matches!(e, DiffEntry::Moved(p) if p == "/a"))
    );
}

#[test]
fn path_guard_preserves_entries() {
    let guard = PathGuard::new(&["nonexistent_binary_xyz123"]);
    let result = guard.ensure_protected_paths("/usr/bin", "/usr/local/bin".to_string());

    assert_eq!(result, "/usr/local/bin");
}
