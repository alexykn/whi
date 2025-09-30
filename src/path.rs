use std::path::PathBuf;

pub struct PathSearcher {
    dirs: Vec<PathBuf>,
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

        PathSearcher { dirs }
    }

    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }
}
