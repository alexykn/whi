use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

pub struct ExecutableCheck<'a> {
    path: &'a Path,
    metadata: Option<fs::Metadata>,
}

impl<'a> ExecutableCheck<'a> {
    #[must_use]
    pub fn new(path: &'a Path) -> Self {
        ExecutableCheck {
            path,
            metadata: None,
        }
    }

    #[must_use]
    pub fn with_metadata(path: &'a Path, metadata: fs::Metadata) -> Self {
        ExecutableCheck {
            path,
            metadata: Some(metadata),
        }
    }

    #[must_use]
    pub fn exists(&self) -> bool {
        self.path.exists()
    }

    #[must_use]
    pub fn is_executable(&self) -> bool {
        // Use cached metadata if available, otherwise fetch it
        let metadata = match &self.metadata {
            Some(m) => m.clone(),
            None => match fs::metadata(self.path) {
                Ok(m) => m,
                Err(_) => return false,
            },
        };

        if !metadata.is_file() {
            return false;
        }

        // Check if any exec bit is set
        let permissions = metadata.permissions();
        let mode = permissions.mode();
        (mode & 0o111) != 0
    }

    #[must_use]
    pub fn get_file_metadata(&self) -> Option<FileMetadata> {
        // Use cached metadata if available, otherwise fetch it
        let metadata = match &self.metadata {
            Some(m) => m.clone(),
            None => fs::metadata(self.path).ok()?,
        };

        Some(FileMetadata {
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.len(),
            mtime: metadata.modified().ok(),
            ctime: metadata.created().ok(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub dev: u64,
    pub ino: u64,
    pub size: u64,
    pub mtime: Option<std::time::SystemTime>,
    pub ctime: Option<std::time::SystemTime>,
}

#[derive(Debug)]
pub struct SearchResult {
    pub path: PathBuf,
    pub canonical_path: Option<PathBuf>,
    pub metadata: Option<FileMetadata>,
    pub path_index: usize,
}
