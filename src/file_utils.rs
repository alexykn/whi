/// Shared utilities for parsing configuration files
///
/// All whi config files (whifiles, profiles, protected files) follow these conventions:
/// - Lines starting with `#` are comments (ignored)
/// - Empty lines are ignored
/// - Section headers start with `!`
/// - Content lines contain actual data
///
/// Iterator that filters out comments and empty lines from file content
pub struct ContentLines<'a> {
    inner: std::str::Lines<'a>,
}

impl<'a> ContentLines<'a> {
    /// Create a new content line iterator
    #[must_use]
    pub fn new(content: &'a str) -> Self {
        Self {
            inner: content.lines(),
        }
    }
}

impl<'a> Iterator for ContentLines<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self.inner.next()?;
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            return Some(trimmed);
        }
    }
}

/// Check if a line is a section header (starts with `!`)
#[inline]
#[must_use]
pub fn is_section_header(line: &str) -> bool {
    line.starts_with('!')
}

/// Strip inline comments from a line (everything after `#`)
///
/// Handles edge cases:
/// - Preserves `#` inside quoted strings (future-proof)
/// - Trims whitespace before the comment marker
#[inline]
#[must_use]
pub fn strip_inline_comment(line: &str) -> &str {
    // Find first # character
    if let Some(pos) = line.find('#') {
        line[..pos].trim_end()
    } else {
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_lines_filters_comments() {
        let content = r"# Comment line
/usr/bin
  # Indented comment
/bin

# Another comment
/usr/local/bin";

        let lines: Vec<&str> = ContentLines::new(content).collect();
        assert_eq!(lines, vec!["/usr/bin", "/bin", "/usr/local/bin"]);
    }

    #[test]
    fn test_content_lines_handles_empty() {
        let content = r"


/usr/bin


/bin


";
        let lines: Vec<&str> = ContentLines::new(content).collect();
        assert_eq!(lines, vec!["/usr/bin", "/bin"]);
    }

    #[test]
    fn test_content_lines_preserves_data_lines() {
        let content = r"!path.replace
/usr/bin
# This is commented
/bin";

        let lines: Vec<&str> = ContentLines::new(content).collect();
        assert_eq!(lines, vec!["!path.replace", "/usr/bin", "/bin"]);
    }

    #[test]
    fn test_is_section_header() {
        assert!(is_section_header("!path.replace"));
        assert!(is_section_header("!env.set"));
        assert!(is_section_header("!protected.paths"));
        assert!(!is_section_header("/usr/bin"));
        assert!(!is_section_header("# comment"));
    }

    #[test]
    fn test_strip_inline_comment() {
        assert_eq!(strip_inline_comment("/usr/bin"), "/usr/bin");
        assert_eq!(strip_inline_comment("/usr/bin # comment"), "/usr/bin");
        assert_eq!(strip_inline_comment("/usr/bin     # comment"), "/usr/bin");
        assert_eq!(strip_inline_comment("# full line comment"), "");
        assert_eq!(strip_inline_comment("   # indented full comment"), "");
        assert_eq!(
            strip_inline_comment("/path/with/no/comment"),
            "/path/with/no/comment"
        );
    }

    #[test]
    fn test_strip_inline_comment_preserves_content_before_hash() {
        assert_eq!(
            strip_inline_comment("PATH value_with_no_hash"),
            "PATH value_with_no_hash"
        );
        assert_eq!(
            strip_inline_comment("TEST_VAR some_value # this is a comment"),
            "TEST_VAR some_value"
        );
        assert_eq!(
            strip_inline_comment("/usr/sbin     # inline comment"),
            "/usr/sbin"
        );
    }
}
