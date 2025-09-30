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

    pub fn clean_duplicates(&self) -> (String, Vec<usize>) {
        let mut seen = std::collections::HashSet::new();
        let mut cleaned = Vec::new();
        let mut removed_indices = Vec::new();

        for (idx, dir) in self.dirs.iter().enumerate() {
            let dir_str = dir.display().to_string();
            if seen.insert(dir_str.clone()) {
                cleaned.push(dir_str);
            } else {
                // Duplicate found - track 1-based index
                removed_indices.push(idx + 1);
            }
        }

        (cleaned.join(":"), removed_indices)
    }

    pub fn delete_entry(&self, idx: usize) -> Result<String, String> {
        let len = self.dirs.len();

        // Validate index (1-based)
        if idx == 0 {
            return Err("index must be >= 1".to_string());
        }
        if idx > len {
            return Err(format!("index out of bounds (PATH has {len} entries)"));
        }

        // Convert to 0-based
        let idx_0 = idx - 1;

        // Create new PATH without this entry
        let mut new_dirs = self.dirs.clone();
        new_dirs.remove(idx_0);

        // Return new PATH string
        Ok(new_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>()
            .join(":"))
    }

    pub fn delete_entries(&self, indices: &[usize]) -> Result<String, String> {
        let len = self.dirs.len();

        // Validate all indices (1-based)
        for &idx in indices {
            if idx == 0 {
                return Err("indices must be >= 1".to_string());
            }
            if idx > len {
                return Err(format!(
                    "index {idx} out of bounds (PATH has {len} entries)"
                ));
            }
        }

        // Sort indices in descending order to delete from highest to lowest
        // This avoids index shifting issues
        let mut sorted_indices: Vec<usize> = indices.to_vec();
        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));

        // Remove duplicates
        sorted_indices.dedup();

        // Create new PATH without these entries
        let mut new_dirs = self.dirs.clone();
        for &idx in &sorted_indices {
            let idx_0 = idx - 1; // Convert to 0-based
            new_dirs.remove(idx_0);
        }

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

    #[test]
    fn test_clean_no_duplicates() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let (result, removed) = searcher.clean_duplicates();
        assert_eq!(result, "/a:/b:/c:/d:/e");
        assert!(removed.is_empty());
    }

    #[test]
    fn test_clean_with_duplicates() {
        let searcher = PathSearcher::new("/a:/b:/c:/b:/d:/a");
        let (result, removed) = searcher.clean_duplicates();
        assert_eq!(result, "/a:/b:/c:/d");
        assert_eq!(removed, vec![4, 6]); // /b at idx 4, /a at idx 6
    }

    #[test]
    fn test_clean_all_same() {
        let searcher = PathSearcher::new("/a:/a:/a");
        let (result, removed) = searcher.clean_duplicates();
        assert_eq!(result, "/a");
        assert_eq!(removed, vec![2, 3]);
    }

    #[test]
    fn test_clean_consecutive_duplicates() {
        let searcher = PathSearcher::new("/a:/a:/b:/b:/c");
        let (result, removed) = searcher.clean_duplicates();
        assert_eq!(result, "/a:/b:/c");
        assert_eq!(removed, vec![2, 4]);
    }

    #[test]
    fn test_clean_empty() {
        let searcher = PathSearcher::new("");
        let (result, removed) = searcher.clean_duplicates();
        assert_eq!(result, ".");
        assert!(removed.is_empty());
    }

    #[test]
    fn test_clean_matches_delete() {
        // Verify that clean and delete produce identical results
        let path = "/a:/b:/c:/b:/d:/a:/e:/c";
        let searcher = PathSearcher::new(path);

        // Get clean result and removed indices
        let (clean_result, removed) = searcher.clean_duplicates();

        // Apply delete with the same indices
        let delete_result = searcher.delete_entries(&removed).unwrap();

        // Results must be identical
        assert_eq!(clean_result, delete_result);
        assert_eq!(removed, vec![4, 6, 8]); // /b at 4, /a at 6, /c at 8
    }

    #[test]
    fn test_delete_first() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.delete_entry(1).unwrap();
        assert_eq!(result, "/b:/c:/d:/e");
    }

    #[test]
    fn test_delete_middle() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.delete_entry(3).unwrap();
        assert_eq!(result, "/a:/b:/d:/e");
    }

    #[test]
    fn test_delete_last() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.delete_entry(5).unwrap();
        assert_eq!(result, "/a:/b:/c:/d");
    }

    #[test]
    fn test_delete_only_entry() {
        let searcher = PathSearcher::new("/a");
        let result = searcher.delete_entry(1).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_delete_zero_index() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.delete_entry(0);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "index must be >= 1");
    }

    #[test]
    fn test_delete_out_of_bounds() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.delete_entry(5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_delete_entries_multiple() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.delete_entries(&[2, 4]).unwrap();
        assert_eq!(result, "/a:/c:/e");
    }

    #[test]
    fn test_delete_entries_unordered() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.delete_entries(&[5, 2, 3]).unwrap();
        assert_eq!(result, "/a:/d");
    }

    #[test]
    fn test_delete_entries_with_duplicates() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.delete_entries(&[2, 2, 4, 4]).unwrap();
        assert_eq!(result, "/a:/c:/e");
    }

    #[test]
    fn test_delete_entries_all() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.delete_entries(&[1, 2, 3]).unwrap();
        assert_eq!(result, "");
    }

    #[test]
    fn test_delete_entries_zero_index() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.delete_entries(&[1, 0, 3]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "indices must be >= 1");
    }

    #[test]
    fn test_delete_entries_out_of_bounds() {
        let searcher = PathSearcher::new("/a:/b:/c");
        let result = searcher.delete_entries(&[1, 5, 2]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("out of bounds"));
    }

    #[test]
    fn test_delete_entries_single() {
        let searcher = PathSearcher::new("/a:/b:/c:/d:/e");
        let result = searcher.delete_entries(&[3]).unwrap();
        assert_eq!(result, "/a:/b:/d:/e");
    }
}
