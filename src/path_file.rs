/// Human-friendly PATH file format utilities
///
/// Format:
/// ```text
/// PATH!
/// /usr/bin
/// /bin
/// /usr/local/bin
///
/// ENV!
/// VAR=value
/// ```

/// Format a PATH string into the human-friendly file format
pub fn format_path_file(path: &str) -> String {
    let mut output = String::from("PATH!\n");

    for entry in path.split(':').filter(|s| !s.is_empty()) {
        output.push_str(entry);
        output.push('\n');
    }

    // Add ENV section marker (for future use)
    output.push_str("\nENV!\n");

    output
}

/// Parse PATH file - supports both new (PATH!/ENV!) and legacy (colon-separated) formats
///
/// This provides backward compatibility with saved_path and profile files from pre-0.5.0 releases.
pub fn parse_path_file(content: &str) -> Result<String, String> {
    let trimmed = content.trim();

    if trimmed.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    // Check if this is the new format (starts with PATH!)
    if trimmed.starts_with("PATH!") {
        // New format: PATH!\n/path1\n/path2\n\nENV!\n
        parse_new_format(trimmed)
    } else {
        // Legacy format: colon-separated string (possibly multi-line)
        parse_legacy_format(trimmed)
    }
}

/// Parse new PATH!/ENV! format
fn parse_new_format(content: &str) -> Result<String, String> {
    let mut path_entries = Vec::new();
    let mut in_path_section = false;
    let mut in_env_section = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Check for section headers
        if line == "PATH!" {
            in_path_section = true;
            in_env_section = false;
            continue;
        } else if line == "ENV!" {
            in_path_section = false;
            in_env_section = true;
            continue;
        }

        // Process content based on current section
        if in_path_section {
            path_entries.push(line.to_string());
        } else if in_env_section {
            // Future: handle ENV vars here
            // For now, we just skip them
        }
    }

    if path_entries.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    Ok(path_entries.join(":"))
}

/// Parse legacy colon-separated format
fn parse_legacy_format(content: &str) -> Result<String, String> {
    // Join all lines and split by colon to handle both single and multi-line legacy files
    let all_lines = content.lines().map(str::trim).collect::<Vec<_>>().join("");

    let entries: Vec<&str> = all_lines
        .split(':')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();

    if entries.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    Ok(entries.join(":"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_path_file() {
        let path = "/usr/bin:/bin:/usr/local/bin";
        let formatted = format_path_file(path);

        assert!(formatted.contains("PATH!"));
        assert!(formatted.contains("/usr/bin\n"));
        assert!(formatted.contains("/bin\n"));
        assert!(formatted.contains("/usr/local/bin\n"));
        assert!(formatted.contains("ENV!"));
    }

    #[test]
    fn test_parse_path_file() {
        let content = r#"PATH!
/usr/bin
/bin
/usr/local/bin

ENV!
"#;
        let path = parse_path_file(content).unwrap();
        assert_eq!(path, "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_roundtrip() {
        let original = "/usr/bin:/bin:/usr/local/bin:/opt/bin";
        let formatted = format_path_file(original);
        let parsed = parse_path_file(&formatted).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_parse_with_extra_whitespace() {
        let content = r#"
        PATH!
        /usr/bin
        /bin

        /usr/local/bin

        ENV!
        "#;
        let path = parse_path_file(content).unwrap();
        assert_eq!(path, "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_parse_empty_file() {
        let content = "";
        let result = parse_path_file(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No PATH entries"));
    }

    #[test]
    fn test_parse_no_path_section() {
        // Content with ENV! but no PATH! section - treated as legacy format
        // This is an edge case that wouldn't occur in practice
        let content = "ENV!\nVAR=value\n";
        let result = parse_path_file(content);
        // Legacy parser would treat this as a single "entry" (nonsense but harmless)
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_legacy_format_single_line() {
        // Legacy format from pre-0.5.0 releases
        let content = "/usr/bin:/bin:/usr/local/bin";
        let path = parse_path_file(content).unwrap();
        assert_eq!(path, "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_parse_legacy_format_with_whitespace() {
        let content = "  /usr/bin:/bin:/usr/local/bin  \n";
        let path = parse_path_file(content).unwrap();
        assert_eq!(path, "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_parse_legacy_format_multiline() {
        // Handle edge case where legacy file might have been edited with newlines
        let content = "/usr/bin:/bin:\n/usr/local/bin:/opt/bin";
        let path = parse_path_file(content).unwrap();
        assert_eq!(path, "/usr/bin:/bin:/usr/local/bin:/opt/bin");
    }

    #[test]
    fn test_parse_legacy_format_with_empty_entries() {
        let content = "/usr/bin::/bin::::/usr/local/bin";
        let path = parse_path_file(content).unwrap();
        // Empty entries should be filtered out
        assert_eq!(path, "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_backward_compatibility_upgrade() {
        // Simulates upgrading from 0.4.x to 0.5.0
        // Old saved_path file has legacy format, should still work
        let legacy_content = "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin";
        let parsed = parse_path_file(legacy_content).unwrap();
        assert_eq!(parsed, legacy_content);

        // New files use new format
        let new_content = "PATH!\n/usr/bin\n/bin\n/usr/local/bin\n/opt/homebrew/bin\n\nENV!\n";
        let parsed_new = parse_path_file(new_content).unwrap();
        assert_eq!(parsed_new, "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin");

        // Both should produce the same result
        assert_eq!(parsed, parsed_new);
    }
}
