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
    } else if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = env::var("HOME")
    {
        return format!("{home}/{rest}");
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

/// Case-insensitive substring search without allocation
/// Returns the index where needle is found in haystack, or None
fn find_ci(haystack: &str, needle: &str, start: usize) -> Option<usize> {
    let haystack_bytes = haystack.as_bytes();
    let needle_bytes = needle.as_bytes();

    if needle_bytes.is_empty() {
        return Some(start);
    }

    let mut pos = start;
    while pos + needle_bytes.len() <= haystack_bytes.len() {
        let matches = haystack_bytes[pos..pos + needle_bytes.len()]
            .iter()
            .zip(needle_bytes)
            .all(|(h, n)| h.eq_ignore_ascii_case(n));

        if matches {
            return Some(pos);
        }
        pos += 1;
    }

    None
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
        let path_str = path.to_string_lossy();
        let mut position = 0;

        for part in &self.query_parts {
            // Find this part starting from current position using case-insensitive search
            if let Some(idx) = find_ci(&path_str, part, position) {
                position = idx + part.len();
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
    s.contains('/')
        || s.starts_with('~')
        || s.starts_with("./")
        || s.starts_with("../")
        || s.contains('\\')
}
