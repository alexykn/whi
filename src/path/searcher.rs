use std::path::PathBuf;

pub struct PathSearcher {
    dirs: Vec<PathBuf>,
    canon_dirs: std::cell::RefCell<Vec<Option<PathBuf>>>,
}

type PathOpResult = Result<String, String>;

/// Validate a `PATH` entry for suspicious or malicious content
fn validate_path_entry(path: &str) -> Result<(), String> {
    // Check for null bytes
    if path.contains('\0') {
        return Err("PATH entry contains null byte".to_string());
    }

    // Check for control characters (except tab which is valid)
    for ch in path.chars() {
        if ch.is_control() && ch != '\t' {
            return Err(format!("PATH entry contains control character: {ch:?}"));
        }
    }

    Ok(())
}

/// Warn about potentially dangerous `PATH` entries
fn warn_suspicious_path(path: &str) {
    // Warn about shell metacharacters that could be dangerous
    const DANGEROUS_CHARS: &[char] = &['$', '`', ';', '&', '|', '<', '>', '(', ')', '{', '}'];

    for &ch in DANGEROUS_CHARS {
        if path.contains(ch) {
            eprintln!("Warning: PATH entry contains shell metacharacter '{ch}': {path}");
            return;
        }
    }

    // Warn about relative paths (but don't reject)
    if !path.starts_with('/') && !path.is_empty() && path != "." {
        eprintln!("Warning: Relative PATH entry detected: {path}");
    }
}

impl PathSearcher {
    #[must_use]
    pub fn new(path_var: &str) -> Self {
        let mut has_empty = false;

        let dirs: Vec<PathBuf> = path_var
            .split(':')
            .filter_map(|s| {
                // Check for empty components
                if s.is_empty() {
                    has_empty = true;
                    return None; // Skip empty components instead of treating as "."
                }

                // Validate entry
                if let Err(e) = validate_path_entry(s) {
                    eprintln!("Warning: Skipping invalid PATH entry: {e}");
                    return None;
                }

                // Warn about suspicious entries
                warn_suspicious_path(s);

                Some(PathBuf::from(s))
            })
            .collect();

        if has_empty {
            eprintln!(
                "Warning: Empty PATH component(s) detected and skipped. Empty components can be a security risk."
            );
        }

        let canon_dirs = std::cell::RefCell::new(vec![None; dirs.len()]);
        PathSearcher { dirs, canon_dirs }
    }

    #[must_use]
    pub fn dirs(&self) -> &[PathBuf] {
        &self.dirs
    }

    /// Get the canonical path for a directory at the given index (0-based)
    fn canonicalize_index(&self, idx: usize) -> Option<PathBuf> {
        let mut cache = self.canon_dirs.borrow_mut();
        if cache[idx].is_none() {
            cache[idx] = std::fs::canonicalize(&self.dirs[idx]).ok();
        }
        cache[idx].clone()
    }

    fn join_dirs(dirs: &[PathBuf]) -> String {
        dirs.iter()
            .map(|dir| dir.display().to_string())
            .collect::<Vec<_>>()
            .join(":")
    }

    fn validate_insert_position(&self, position: usize) -> Result<usize, String> {
        if position == 0 {
            return Err("Position must be >= 1".to_string());
        }

        Ok((position - 1).min(self.dirs.len()))
    }

    fn validate_index(&self, index: usize, label: &str) -> Result<usize, String> {
        if index == 0 {
            return Err(format!("Invalid index: {index} ({label} must be >= 1)"));
        }

        let len = self.dirs.len();
        if index > len {
            return Err(format!(
                "Index {index} out of bounds (PATH has {len} entries)"
            ));
        }

        Ok(index - 1)
    }

    fn validate_move_indices(&self, from: usize, to: usize) -> Result<(usize, usize), String> {
        let len = self.dirs.len();

        if from == 0 || to == 0 {
            return Err(format!(
                "Invalid index: indices must be >= 1 (got from={from}, to={to})"
            ));
        }
        if from > len {
            return Err(format!(
                "Index {from} out of bounds (PATH has {len} entries)"
            ));
        }
        if to > len {
            return Err(format!("Index {to} out of bounds (PATH has {len} entries)"));
        }

        Ok((from - 1, to - 1))
    }

    fn validate_swap_indices(&self, first: usize, second: usize) -> Result<(usize, usize), String> {
        let len = self.dirs.len();

        if first == 0 || second == 0 {
            return Err(format!(
                "Invalid index: indices must be >= 1 (got idx1={first}, idx2={second})"
            ));
        }
        if first > len {
            return Err(format!(
                "Index {first} out of bounds (PATH has {len} entries)"
            ));
        }
        if second > len {
            return Err(format!(
                "Index {second} out of bounds (PATH has {len} entries)"
            ));
        }

        Ok((first - 1, second - 1))
    }

    fn canonical_search_path(path: &std::path::Path) -> PathBuf {
        std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
    }

    fn matches_exact_or_canonical(
        &self,
        idx: usize,
        dir: &std::path::Path,
        path: &std::path::Path,
        canonical_search: &std::path::Path,
    ) -> bool {
        if dir == path || dir == canonical_search {
            return true;
        }

        self.canonicalize_index(idx)
            .is_some_and(|canonical_dir| canonical_dir == canonical_search)
    }

    /// Check if a path already exists in `PATH`
    #[must_use]
    pub fn contains(&self, path: &std::path::Path) -> bool {
        self.find_path_index(path).is_some()
    }

    /// Insert a path at the given position (1-based), mutating this `PathSearcher`
    /// Returns Err if position is invalid
    pub fn insert_at(&mut self, path: &std::path::Path, position: usize) -> Result<(), String> {
        let path_buf = path.to_path_buf();
        let insert_idx = self.validate_insert_position(position)?;

        self.dirs.insert(insert_idx, path_buf);
        self.canon_dirs.borrow_mut().insert(insert_idx, None);
        Ok(())
    }

    pub fn move_entry(&self, from: usize, to: usize) -> PathOpResult {
        let (from_idx, to_idx) = self.validate_move_indices(from, to)?;
        let mut new_dirs = self.dirs.clone();
        let item = new_dirs.remove(from_idx);
        new_dirs.insert(to_idx, item);
        Ok(Self::join_dirs(&new_dirs))
    }

    pub fn swap_entries(&self, idx1: usize, idx2: usize) -> PathOpResult {
        let (idx1_0, idx2_0) = self.validate_swap_indices(idx1, idx2)?;
        let mut new_dirs = self.dirs.clone();
        new_dirs.swap(idx1_0, idx2_0);
        Ok(Self::join_dirs(&new_dirs))
    }

    #[must_use]
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

    pub fn delete_entry(&self, idx: usize) -> PathOpResult {
        let idx_0 = self.validate_index(idx, "index")?;
        let mut new_dirs = self.dirs.clone();
        new_dirs.remove(idx_0);
        Ok(Self::join_dirs(&new_dirs))
    }

    pub fn delete_entries(&self, indices: &[usize]) -> PathOpResult {
        for &idx in indices {
            self.validate_index(idx, "indices")?;
        }

        let mut sorted_indices: Vec<usize> = indices.to_vec();
        sorted_indices.sort_unstable_by(|a, b| b.cmp(a));
        sorted_indices.dedup();

        let mut new_dirs = self.dirs.clone();
        for &idx in &sorted_indices {
            new_dirs.remove(idx - 1);
        }
        Ok(Self::join_dirs(&new_dirs))
    }

    /// Add a new directory to `PATH` if not already present at the beginning
    /// Returns the new `PATH` string and the index where it was added (1-based)
    pub fn add_path(&self, path: &std::path::Path) -> Result<(String, usize), String> {
        match self.add_path_at_position(path, 1) {
            Ok(new_path) => Ok((new_path, 1)),
            Err(e) => Err(e),
        }
    }

    /// Add a new directory to `PATH` at a specific position if not already present
    /// Returns the new `PATH` string (1-based position)
    pub fn add_path_at_position(
        &self,
        path: &std::path::Path,
        position: usize,
    ) -> Result<String, String> {
        let path_buf = path.to_path_buf();

        if self.find_path_index(&path_buf).is_some() {
            return Ok(self.to_path_string());
        }

        let mut new_dirs = self.dirs.clone();
        let insert_idx = self.validate_insert_position(position)?;
        new_dirs.insert(insert_idx, path_buf);
        Ok(Self::join_dirs(&new_dirs))
    }

    /// Find the index of an exact path match (1-based)
    #[must_use]
    pub fn find_path_index(&self, path: &std::path::Path) -> Option<usize> {
        let canonical_search = Self::canonical_search_path(path);

        for (idx, dir) in self.dirs.iter().enumerate() {
            if self.matches_exact_or_canonical(idx, dir, path, &canonical_search) {
                return Some(idx + 1);
            }
        }

        None
    }

    /// Find all indices matching a fuzzy pattern
    #[must_use]
    pub fn find_fuzzy_indices(
        &self,
        pattern: &str,
        executable_name: Option<&str>,
    ) -> Vec<(usize, &PathBuf)> {
        use crate::path::fuzzy::FuzzyMatcher;

        let matcher = FuzzyMatcher::new(pattern);
        let mut fuzzy_results = Vec::new();

        for (idx, dir) in self.dirs.iter().enumerate() {
            if matcher.matches(dir) {
                // If executable specified, check it exists
                if let Some(name) = executable_name
                    && !self.has_executable(dir, name)
                {
                    continue;
                }

                fuzzy_results.push((idx + 1, dir)); // 1-based index
            }
        }

        // Sort by match quality (shorter paths first)
        fuzzy_results.sort_by_key(|(_, path)| path.as_os_str().len());

        fuzzy_results
    }

    /// Delete a `PATH` entry by exact path match
    #[allow(dead_code)]
    pub fn delete_by_path(&self, path: &std::path::Path) -> Result<String, String> {
        if let Some(idx) = self.find_path_index(path) {
            self.delete_entry(idx)
        } else {
            Err(format!("Path not found in PATH: {}", path.display()))
        }
    }

    /// Check if an executable exists in a directory
    #[must_use]
    pub fn has_executable(&self, dir: &std::path::Path, name: &str) -> bool {
        use crate::search::result::ExecutableCheck;

        let exec_path = dir.join(name);
        exec_path.exists() && ExecutableCheck::new(&exec_path).is_executable()
    }

    /// Convert current dirs to `PATH` string
    #[must_use]
    pub fn to_path_string(&self) -> String {
        Self::join_dirs(&self.dirs)
    }
}
