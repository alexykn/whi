use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Atomic file writer that uses temp file + rename pattern
/// Ensures either complete success or no changes (no partial writes)
pub struct AtomicFile {
    target: PathBuf,
    temp: PathBuf,
    file: Option<File>,
}

impl AtomicFile {
    /// Create a new atomic file writer for the given path
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let target = path.as_ref().to_path_buf();

        // Create temp file with unique name based on PID
        let temp = target.with_extension(format!("tmp.{}", std::process::id()));

        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)?;

        Ok(AtomicFile {
            target,
            temp,
            file: Some(file),
        })
    }

    /// Commit the changes by atomically renaming temp file to target
    pub fn commit(mut self) -> io::Result<()> {
        // Ensure all data is written to disk
        if let Some(ref file) = self.file {
            file.sync_all()?;
        }

        // Close the file
        self.file = None;

        // Atomic rename - either succeeds completely or not at all
        let result = fs::rename(&self.temp, &self.target);

        // Forget self to prevent Drop from trying to remove the file
        if result.is_ok() {
            std::mem::forget(self);
        }

        result
    }

    /// Cancel the operation and remove the temp file
    #[allow(dead_code)]
    pub fn cancel(mut self) -> io::Result<()> {
        // Close the file
        self.file = None;

        let result = fs::remove_file(&self.temp);

        // Forget self to prevent double-deletion in Drop
        std::mem::forget(self);

        result
    }
}

impl Write for AtomicFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match self.file.as_mut() {
            Some(file) => file.write(buf),
            None => Err(io::Error::other("File already closed")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.flush(),
            None => Ok(()),
        }
    }
}

// Implement Drop to ensure temp file is cleaned up if not committed
impl Drop for AtomicFile {
    fn drop(&mut self) {
        // If temp file still exists, try to remove it
        // Ignore errors since we're in Drop
        let _ = fs::remove_file(&self.temp);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_atomic_write_commit() {
        let test_path = "/tmp/whi_test_atomic_commit.txt";

        // Ensure clean state
        let _ = fs::remove_file(test_path);

        // Write and commit
        {
            let mut atomic = AtomicFile::new(test_path).unwrap();
            atomic.write_all(b"test content").unwrap();
            atomic.commit().unwrap();
        }

        // Verify file exists with correct content
        assert!(Path::new(test_path).exists());
        let content = fs::read_to_string(test_path).unwrap();
        assert_eq!(content, "test content");

        // Verify no temp file left
        let temp_path = format!("{}.tmp.{}", test_path, std::process::id());
        assert!(!Path::new(&temp_path).exists());

        // Cleanup
        fs::remove_file(test_path).unwrap();
    }

    #[test]
    fn test_atomic_write_cancel() {
        let test_path = "/tmp/whi_test_atomic_cancel.txt";

        // Ensure clean state
        let _ = fs::remove_file(test_path);

        // Write and cancel
        {
            let mut atomic = AtomicFile::new(test_path).unwrap();
            atomic.write_all(b"test content").unwrap();
            atomic.cancel().unwrap();
        }

        // Verify target file was not created
        assert!(!Path::new(test_path).exists());

        // Verify temp file was removed
        let temp_path = format!("{}.tmp.{}", test_path, std::process::id());
        assert!(!Path::new(&temp_path).exists());
    }

    #[test]
    fn test_atomic_write_drop_cleanup() {
        let test_path = "/tmp/whi_test_atomic_drop.txt";

        // Ensure clean state
        let _ = fs::remove_file(test_path);

        // Write but drop without commit
        {
            let mut atomic = AtomicFile::new(test_path).unwrap();
            atomic.write_all(b"test content").unwrap();
            // Drop without commit or cancel
        }

        // Verify target file was not created
        assert!(!Path::new(test_path).exists());

        // Verify temp file was cleaned up by Drop
        let temp_path = format!("{}.tmp.{}", test_path, std::process::id());
        assert!(!Path::new(&temp_path).exists());
    }

    #[test]
    fn test_atomic_write_overwrites_existing() {
        let test_path = "/tmp/whi_test_atomic_overwrite.txt";

        // Create initial file
        fs::write(test_path, b"initial content").unwrap();

        // Overwrite atomically
        {
            let mut atomic = AtomicFile::new(test_path).unwrap();
            atomic.write_all(b"new content").unwrap();
            atomic.commit().unwrap();
        }

        // Verify content was updated
        let content = fs::read_to_string(test_path).unwrap();
        assert_eq!(content, "new content");

        // Cleanup
        fs::remove_file(test_path).unwrap();
    }
}
