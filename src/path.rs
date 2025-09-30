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

    pub fn move_entry(&self, from: usize, to: usize) -> Result<String, String> {
        let len = self.dirs.len();

        // Validate indices (1-based)
        if from == 0 || to == 0 {
            return Err("indices must be >= 1".to_string());
        }
        if from > len || to > len {
            return Err(format!("index out of bounds (PATH has {len} entries)"));
        }

        // Convert to 0-based
        let from_idx = from - 1;
        let to_idx = to - 1;

        // Create new ordering
        let mut new_dirs = self.dirs.clone();
        let item = new_dirs.remove(from_idx);
        new_dirs.insert(to_idx, item);

        // Return new PATH string
        Ok(new_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(":"))
    }

    pub fn swap_entries(&self, idx1: usize, idx2: usize) -> Result<String, String> {
        let len = self.dirs.len();

        // Validate indices (1-based)
        if idx1 == 0 || idx2 == 0 {
            return Err("indices must be >= 1".to_string());
        }
        if idx1 > len || idx2 > len {
            return Err(format!("index out of bounds (PATH has {len} entries)"));
        }

        // Convert to 0-based
        let idx1_0 = idx1 - 1;
        let idx2_0 = idx2 - 1;

        // Create new ordering with swapped entries
        let mut new_dirs = self.dirs.clone();
        new_dirs.swap(idx1_0, idx2_0);

        // Return new PATH string
        Ok(new_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(":"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_move_entry_forward() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.move_entry(5, 2).unwrap();
        assert_eq!(result, "/a:/e:/b:/c:/d");
    }

    #[test]
    fn test_move_entry_backward() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.move_entry(2, 4).unwrap();
        assert_eq!(result, "/a:/c:/d:/b:/e");
    }

    #[test]
    fn test_move_entry_to_first() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.move_entry(4, 1).unwrap();
        assert_eq!(result, "/d:/a:/b:/c:/e");
    }

    #[test]
    fn test_move_entry_to_last() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.move_entry(2, 5).unwrap();
        assert_eq!(result, "/a:/c:/d:/e:/b");
    }

    #[test]
    fn test_move_entry_same_position() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.move_entry(3, 3).unwrap();
        assert_eq!(result, "/a:/b:/c:/d:/e");
    }

    #[test]
    fn test_move_entry_zero_index() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.move_entry(0, 2);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "indices must be >= 1");
    }

    #[test]
    fn test_move_entry_out_of_bounds() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.move_entry(1, 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_swap_entries_basic() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.swap_entries(2, 4).unwrap();
        assert_eq!(result, "/a:/d:/c:/b:/e");
    }

    #[test]
    fn test_swap_entries_same_index() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.swap_entries(3, 3).unwrap();
        assert_eq!(result, "/a:/b:/c:/d:/e");
    }

    #[test]
    fn test_swap_entries_first_and_last() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.swap_entries(1, 5).unwrap();
        assert_eq!(result, "/e:/b:/c:/d:/a");
    }

    #[test]
    fn test_swap_entries_zero_index() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.swap_entries(0, 2);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "indices must be >= 1");
    }

    #[test]
    fn test_swap_entries_out_of_bounds() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.swap_entries(2, 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }
}
