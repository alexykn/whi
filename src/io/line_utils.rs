/// Shared utilities for parsing configuration files
///
/// All whi config files (path files, profiles, protected files) follow these conventions:
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
#[inline]
#[must_use]
pub fn strip_inline_comment(line: &str) -> &str {
    if let Some(pos) = line.find('#') {
        line[..pos].trim_end()
    } else {
        line
    }
}
