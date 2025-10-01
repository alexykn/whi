use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Expands tilde notation in paths
#[must_use]
pub fn expand_tilde(path: &str) -> String {
    if path == "~" {
        if let Ok(home) = env::var("HOME") {
            return home;
        }
    } else if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = env::var("HOME") {
            return format!("{home}/{rest}");
        }
    }
    path.to_string()
}

/// Resolves a path string to an absolute `PathBuf`
pub fn resolve_path(path_str: &str, cwd: &Path) -> Result<PathBuf, String> {
    // First expand tilde
    let expanded = expand_tilde(path_str);
    let path = Path::new(&expanded);

    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        // Make relative to current directory
        let absolute = cwd.join(path);

        // Try to canonicalize if it exists, otherwise just clean it up
        if absolute.exists() {
            fs::canonicalize(&absolute).map_err(|e| format!("Failed to canonicalize path: {e}"))
        } else {
            // Clean up path (remove ./ and ../ where possible)
            Ok(normalize_path(&absolute))
        }
    }
}

/// Normalizes a path by resolving . and .. components
fn normalize_path(path: &Path) -> PathBuf {
    let mut components = Vec::new();

    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                components.pop();
            }
            std::path::Component::CurDir => {
                // Skip
            }
            c => components.push(c),
        }
    }

    components.iter().collect()
}

/// Performs fuzzy matching on a path using zoxide-style rules
pub struct FuzzyMatcher {
    query_parts: Vec<String>,
}

impl FuzzyMatcher {
    pub fn new(query: &str) -> Self {
        // Split by whitespace and convert to lowercase for case-insensitive matching
        let query_parts: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();

        FuzzyMatcher { query_parts }
    }

    /// Check if a path matches the fuzzy query
    #[must_use]
    pub fn matches(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy().to_lowercase();
        let mut position = 0;

        for part in &self.query_parts {
            // Find this part starting from current position
            if let Some(idx) = path_str[position..].find(part) {
                position += idx + part.len();
            } else {
                return false; // Part not found
            }
        }

        true
    }

    /// Score a match (lower is better, 0 is exact match)
    #[allow(dead_code)]
    #[must_use]
    pub fn score(&self, path: &Path) -> Option<usize> {
        if !self.matches(path) {
            return None;
        }

        // Simple scoring: shorter paths that match are better
        let path_str = path.to_string_lossy();
        Some(path_str.len())
    }
}

/// Determines if a string looks like an exact path (not a fuzzy pattern)
#[must_use]
pub fn looks_like_exact_path(s: &str) -> bool {
    s.contains('/') || s.starts_with('~') || s.starts_with('.') || s.contains('\\')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_tilde_with_slash() {
        env::set_var("HOME", "/home/testuser");
        assert_eq!(expand_tilde("~/bin"), "/home/testuser/bin");
        assert_eq!(expand_tilde("~/"), "/home/testuser/");
    }

    #[test]
    fn test_expand_tilde_bare() {
        env::set_var("HOME", "/home/testuser");
        assert_eq!(expand_tilde("~"), "/home/testuser");
    }

    #[test]
    fn test_expand_tilde_no_match() {
        assert_eq!(expand_tilde("/usr/local"), "/usr/local");
        assert_eq!(expand_tilde("~user/bin"), "~user/bin"); // Not supported
    }

    #[test]
    fn test_fuzzy_matcher_basic() {
        let matcher = FuzzyMatcher::new("cargo bin");
        assert!(matcher.matches(Path::new("/Users/alxknt/.cargo/bin")));
        assert!(matcher.matches(Path::new("/home/user/.cargo/tools/bin")));
        assert!(!matcher.matches(Path::new("/usr/local/bin")));
    }

    #[test]
    fn test_fuzzy_matcher_order() {
        let matcher = FuzzyMatcher::new("github whi");
        assert!(matcher.matches(Path::new("/Users/alxknt/github/whi/target")));
        assert!(!matcher.matches(Path::new("/Users/alxknt/whi/github"))); // Wrong order
    }

    #[test]
    fn test_fuzzy_matcher_case_insensitive() {
        let matcher = FuzzyMatcher::new("USERS CARGO");
        assert!(matcher.matches(Path::new("/users/alxknt/.cargo/bin")));
    }

    #[test]
    fn test_looks_like_exact_path() {
        assert!(looks_like_exact_path("/usr/bin"));
        assert!(looks_like_exact_path("~/bin"));
        assert!(looks_like_exact_path("./target"));
        assert!(looks_like_exact_path("../bin"));
        assert!(!looks_like_exact_path("cargo"));
        assert!(!looks_like_exact_path("users cargo"));
    }

    #[test]
    fn test_normalize_path() {
        let path = PathBuf::from("/usr/local/../bin/./test");
        let normalized = normalize_path(&path);
        assert_eq!(normalized, PathBuf::from("/usr/bin/test"));
    }
}
