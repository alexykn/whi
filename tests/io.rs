use std::fs;
use std::io::Write;
use std::path::Path;

use tempfile::TempDir;
use whi::io::atomic_file::AtomicFile;
use whi::io::line_utils::{ContentLines, strip_inline_comment};

#[test]
fn line_utils_filters_and_strips_comments() {
    let content = r"# Comment line
/usr/bin
  # Indented comment
/bin

# Another comment
/usr/local/bin";

    let lines: Vec<&str> = ContentLines::new(content).collect();
    assert_eq!(lines, vec!["/usr/bin", "/bin", "/usr/local/bin"]);

    assert_eq!(strip_inline_comment("/usr/bin # comment"), "/usr/bin");
    assert_eq!(strip_inline_comment("   # indented full comment"), "");
}

#[test]
fn atomic_file_commit_and_cancel() {
    let dir = TempDir::new().unwrap();
    let commit_path = dir.path().join("commit.txt");
    let cancel_path = dir.path().join("cancel.txt");

    {
        let mut atomic = AtomicFile::new(&commit_path).unwrap();
        atomic.write_all(b"test content").unwrap();
        atomic.commit().unwrap();
    }

    assert!(Path::new(&commit_path).exists());
    assert_eq!(fs::read_to_string(&commit_path).unwrap(), "test content");

    {
        let mut atomic = AtomicFile::new(&cancel_path).unwrap();
        atomic.write_all(b"test content").unwrap();
        atomic.cancel().unwrap();
    }

    assert!(!Path::new(&cancel_path).exists());
}

#[test]
fn atomic_file_overwrite_existing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("overwrite.txt");

    fs::write(&path, b"initial content").unwrap();

    {
        let mut atomic = AtomicFile::new(&path).unwrap();
        atomic.write_all(b"new content").unwrap();
        atomic.commit().unwrap();
    }

    assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
}
