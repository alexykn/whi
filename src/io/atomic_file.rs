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
        if let Some(ref file) = self.file {
            file.sync_all()?;
        }

        self.file = None;
        let result = fs::rename(&self.temp, &self.target);

        if result.is_ok() {
            std::mem::forget(self);
        }

        result
    }

    /// Cancel the operation and remove the temp file
    #[allow(dead_code)]
    pub fn cancel(mut self) -> io::Result<()> {
        self.file = None;

        let result = fs::remove_file(&self.temp);
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

impl Drop for AtomicFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.temp);
    }
}
