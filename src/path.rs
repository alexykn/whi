use std::path::PathBuf;

pub struct PathSearcher {
    dirs: Vec<PathBuf>,
    #[allow(dead_code)]
    original: String,
}

impl PathSearcher {
    pub fn new(path_var: &str) -> Self {
        let dirs: Vec<PathBuf> = path_var
            .split(':')
            .map(|s| {
                if s.is_empty() {
                    PathBuf::from(".")
                } else {
                    PathBuf::from(s)
                }
            })
            .collect();

        PathSearcher {
            dirs,
            original: path_var.to_string(),
        }
    }

    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    #[allow(dead_code)]
    pub fn original(&self) -> &str {
        &self.original
    }
}
