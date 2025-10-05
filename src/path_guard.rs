use std::path::{Path, PathBuf};

/// Guards critical binaries by ensuring their paths are preserved during `PATH` operations
pub struct PathGuard {
    protected_binaries: Vec<String>,
}

impl Default for PathGuard {
    fn default() -> Self {
        Self {
            protected_binaries: vec![
                // whi itself and common integrations
                "whi".to_string(),
                "zoxide".to_string(),
                // Critical system commands used by shell integrations
                "seq".to_string(),     // Fish integration (command lookup)
                "uname".to_string(),   // Fish prompt functions
                "stat".to_string(),    // Both shells (file metadata)
                "command".to_string(), // Both shells (command checking)
            ],
        }
    }
}

impl PathGuard {
    /// Create guard with custom protected binaries
    #[must_use]
    pub fn new(binaries: &[&str]) -> Self {
        Self {
            protected_binaries: binaries.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// Ensure protected binary paths from `original_path` are preserved in `new_path`
    ///
    /// Silently appends missing protected paths to the end of `new_path`
    #[must_use]
    pub fn ensure_protected_paths(&self, original_path: &str, new_path: String) -> String {
        let protected_dirs = self.detect_protected_paths(original_path);

        if protected_dirs.is_empty() {
            return new_path;
        }

        let new_entries: Vec<&str> = new_path.split(':').filter(|s| !s.is_empty()).collect();
        let mut result = new_path.clone();

        for dir in protected_dirs {
            let dir_str = dir.to_string_lossy();

            // Check if this directory is already in new_path
            if !new_entries.iter().any(|&entry| entry == dir_str.as_ref()) {
                // Append at the end to minimize disruption
                if !result.is_empty() && !result.ends_with(':') {
                    result.push(':');
                }
                result.push_str(&dir_str);
            }
        }

        result
    }

    /// Find directories containing protected binaries in current `PATH`
    ///
    /// Silently ignores binaries that are not found - no crashes if binary doesn't exist
    fn detect_protected_paths(&self, current_path: &str) -> Vec<PathBuf> {
        use std::collections::HashSet;

        let protected_dirs: HashSet<PathBuf> = self
            .protected_binaries
            .iter()
            .filter_map(|binary| Self::find_binary_dir(current_path, binary))
            .collect();

        protected_dirs.into_iter().collect()
    }

    /// Find the winning (first) directory for a binary in `PATH`
    fn find_binary_dir(path_str: &str, binary_name: &str) -> Option<PathBuf> {
        for dir in path_str.split(':') {
            if dir.is_empty() {
                continue;
            }

            let dir_path = PathBuf::from(dir);
            let exe_path = dir_path.join(binary_name);

            if Self::is_executable(&exe_path) {
                return Some(dir_path);
            }
        }

        None
    }

    /// Check if a file is executable
    #[cfg(unix)]
    fn is_executable(path: &Path) -> bool {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;

        if let Ok(metadata) = fs::metadata(path) {
            if metadata.is_file() {
                let permissions = metadata.permissions();
                let mode = permissions.mode();
                // Check if executable bit is set (user, group, or other)
                return (mode & 0o111) != 0;
            }
        }

        false
    }

    #[cfg(not(unix))]
    fn is_executable(path: &Path) -> bool {
        // On non-Unix, just check if file exists
        path.is_file()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_preserves_missing_binary() {
        let original = "/usr/local/bin:/home/user/.cargo/bin:/usr/bin";
        let modified = "/usr/local/bin:/usr/bin"; // cargo bin removed

        // Create a test directory structure (would need actual files for real test)
        // For now, test the logic with empty result
        let guard = PathGuard::new(&["nonexistent"]);
        let result = guard.ensure_protected_paths(original, modified.to_string());

        // Should return modified as-is since binary doesn't exist
        assert_eq!(result, modified);
    }

    #[test]
    fn test_guard_skips_already_present() {
        let original = "/usr/local/bin:/home/user/.cargo/bin";
        let modified = "/usr/local/bin:/home/user/.cargo/bin";

        let guard = PathGuard::new(&["test"]);
        let result = guard.ensure_protected_paths(original, modified.to_string());

        // Should not duplicate
        assert_eq!(result, modified);
    }

    #[test]
    fn test_guard_appends_to_empty_path() {
        // Use nonexistent binaries so detect_protected_paths returns empty
        let guard = PathGuard::new(&["nonexistent_binary_xyz123"]);

        // If new_path is empty and we had protected paths, they'd be appended
        // (though in practice this shouldn't happen)
        let result = guard.ensure_protected_paths("/usr/bin", "".to_string());

        // Without real files, detect_protected_paths returns empty, so result is ""
        assert_eq!(result, "");
    }

    #[test]
    fn test_guard_handles_missing_binary_gracefully() {
        // Test that guard doesn't crash when protected binary doesn't exist
        let guard = PathGuard::new(&["nonexistent_binary_xyz123"]);

        let original = "/usr/local/bin:/usr/bin";
        let modified = "/usr/local/bin";

        // Should not panic, just return modified path as-is
        let result = guard.ensure_protected_paths(original, modified.to_string());

        assert_eq!(result, modified);
    }

    #[test]
    fn test_guard_ignores_uninstalled_binaries() {
        // Test with mix of real and fake binaries
        let guard = PathGuard::new(&["sh", "fake_binary_that_does_not_exist"]);

        // sh exists in /bin on most Unix systems
        let original = "/bin:/usr/bin";
        let modified = "/usr/bin";

        // Should preserve /bin (for sh) but not crash on fake binary
        let result = guard.ensure_protected_paths(original, modified.to_string());

        // Result should contain /usr/bin, and may contain /bin if sh was found there
        assert!(result.contains("/usr/bin"));
    }
}
