use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub struct ExecutableCheck<'a> {
    path: &'a Path,
}

impl<'a> ExecutableCheck<'a> {
    pub fn new(path: &'a Path) -> Self {
        ExecutableCheck { path }
    }

    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    pub fn is_executable(&self) -> bool {
        // Check if it's a regular file or symlink to one
        let Ok(metadata) = fs::metadata(self.path) else {
            return false;
        };

        if !metadata.is_file() {
            return false;
        }

        // Check if any exec bit is set
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        (mode & 0o111) != 0
    }

    pub fn get_file_metadata(&self) -> Option<FileMetadata> {
        let metadata = fs::metadata(self.path).ok()?;

        Some(FileMetadata {
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.len(),
            mtime: metadata.modified().ok(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub dev: u64,
    pub ino: u64,
    pub size: u64,
    pub mtime: Option<std::time::SystemTime>,
}

#[derive(Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub canonical_path: Option<PathBuf>,
    pub is_executable: bool,
    pub metadata: Option<FileMetadata>,
    pub path_index: usize,
}
