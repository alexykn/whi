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
/// Parsed path file containing both PATH and ENV vars
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPathFile {
    pub path: String,
    pub env_vars: Vec<(String, String)>,
}

/// Format a PATH string into the human-friendly file format
#[must_use]
pub fn format_path_file(path: &str) -> String {
    format_path_file_with_env(path, &[])
}

/// Format a PATH string with optional ENV vars into the human-friendly file format
#[must_use]
pub fn format_path_file_with_env(path: &str, env_vars: &[(String, String)]) -> String {
    let mut output = String::from("PATH!\n");

    for entry in path.split(':').filter(|s| !s.is_empty()) {
        output.push_str(entry);
        output.push('\n');
    }

    // Add ENV section
    output.push_str("\nENV!\n");
    for (key, value) in env_vars {
        output.push_str(key);
        output.push(' ');
        output.push_str(value);
        output.push('\n');
    }

    output
}

/// Parse PATH file - supports both new (PATH!/ENV!) and legacy (colon-separated) formats
///
/// Returns `ParsedPathFile` with path string and optional env vars.
/// This provides backward compatibility with `saved_path` and profile files from pre-0.5.2 releases.
pub fn parse_path_file(content: &str) -> Result<ParsedPathFile, String> {
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
fn parse_new_format(content: &str) -> Result<ParsedPathFile, String> {
    let mut path_entries = Vec::new();
    let mut env_vars = Vec::new();
    let mut in_path_section = false;
    let mut in_env_section = false;

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
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
            // Parse ENV var: KEY value (space-separated, fish-style)
            if let Some(space_idx) = line.find(char::is_whitespace) {
                let key = line[..space_idx].to_string();
                let value = line[space_idx..].trim().to_string();
                env_vars.push((key, value));
            } else {
                // No value, set to empty string
                env_vars.push((line.to_string(), String::new()));
            }
        }
    }

    if path_entries.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    Ok(ParsedPathFile {
        path: path_entries.join(":"),
        env_vars,
    })
}

/// Parse legacy colon-separated format
fn parse_legacy_format(content: &str) -> Result<ParsedPathFile, String> {
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

    Ok(ParsedPathFile {
        path: entries.join(":"),
        env_vars: Vec::new(), // Legacy format has no ENV vars
    })
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
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin");
        assert!(parsed.env_vars.is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let original = "/usr/bin:/bin:/usr/local/bin:/opt/bin";
        let formatted = format_path_file(original);
        let parsed = parse_path_file(&formatted).unwrap();
        assert_eq!(parsed.path, original);
        assert!(parsed.env_vars.is_empty());
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
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin");
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
        // Legacy format from pre-0.5.2 releases
        let content = "/usr/bin:/bin:/usr/local/bin";
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin");
        assert!(parsed.env_vars.is_empty());
    }

    #[test]
    fn test_parse_legacy_format_with_whitespace() {
        let content = "  /usr/bin:/bin:/usr/local/bin  \n";
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_parse_legacy_format_multiline() {
        // Handle edge case where legacy file might have been edited with newlines
        let content = "/usr/bin:/bin:\n/usr/local/bin:/opt/bin";
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin:/opt/bin");
    }

    #[test]
    fn test_parse_legacy_format_with_empty_entries() {
        let content = "/usr/bin::/bin::::/usr/local/bin";
        let parsed = parse_path_file(content).unwrap();
        // Empty entries should be filtered out
        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_backward_compatibility_upgrade() {
        // Simulates upgrading from 0.4.x to 0.5.2
        // Old saved_path file has legacy format, should still work
        let legacy_content = "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin";
        let parsed = parse_path_file(legacy_content).unwrap();
        assert_eq!(parsed.path, legacy_content);

        // New files use new format
        let new_content = "PATH!\n/usr/bin\n/bin\n/usr/local/bin\n/opt/homebrew/bin\n\nENV!\n";
        let parsed_new = parse_path_file(new_content).unwrap();
        assert_eq!(
            parsed_new.path,
            "/usr/bin:/bin:/usr/local/bin:/opt/homebrew/bin"
        );

        // Both should produce the same result
        assert_eq!(parsed.path, parsed_new.path);
    }

    #[test]
    fn test_parse_env_vars() {
        let content = r#"PATH!
/usr/bin
/bin

ENV!
RUST_LOG debug
MY_VAR hello world
EMPTY_VAR
"#;
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.path, "/usr/bin:/bin");
        assert_eq!(parsed.env_vars.len(), 3);
        assert_eq!(
            parsed.env_vars[0],
            ("RUST_LOG".to_string(), "debug".to_string())
        );
        assert_eq!(
            parsed.env_vars[1],
            ("MY_VAR".to_string(), "hello world".to_string())
        );
        assert_eq!(parsed.env_vars[2], ("EMPTY_VAR".to_string(), String::new()));
    }

    #[test]
    fn test_parse_env_vars_with_comments() {
        let content = r#"PATH!
/usr/bin
/bin

ENV!
# This is a comment
RUST_LOG debug
# Another comment
MY_VAR value
"#;
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.env_vars.len(), 2);
        assert_eq!(
            parsed.env_vars[0],
            ("RUST_LOG".to_string(), "debug".to_string())
        );
        assert_eq!(
            parsed.env_vars[1],
            ("MY_VAR".to_string(), "value".to_string())
        );
    }

    #[test]
    fn test_format_with_env_vars() {
        let env_vars = vec![
            ("RUST_LOG".to_string(), "debug".to_string()),
            ("MY_VAR".to_string(), "hello world".to_string()),
        ];
        let formatted = format_path_file_with_env("/usr/bin:/bin", &env_vars);

        assert!(formatted.contains("PATH!"));
        assert!(formatted.contains("/usr/bin\n"));
        assert!(formatted.contains("/bin\n"));
        assert!(formatted.contains("ENV!"));
        assert!(formatted.contains("RUST_LOG debug\n"));
        assert!(formatted.contains("MY_VAR hello world\n"));
    }

    #[test]
    fn test_roundtrip_with_env_vars() {
        let env_vars = vec![
            ("RUST_LOG".to_string(), "trace".to_string()),
            ("PATH_VAR".to_string(), "/some/path".to_string()),
        ];
        let formatted = format_path_file_with_env("/usr/bin:/bin:/usr/local/bin", &env_vars);
        let parsed = parse_path_file(&formatted).unwrap();

        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin");
        assert_eq!(parsed.env_vars, env_vars);
    }

    #[test]
    fn test_legacy_format_no_env_vars() {
        let legacy_content = "/usr/bin:/bin:/usr/local/bin";
        let parsed = parse_path_file(legacy_content).unwrap();

        assert_eq!(parsed.path, "/usr/bin:/bin:/usr/local/bin");
        assert!(parsed.env_vars.is_empty());
    }

    #[test]
    fn test_env_var_with_special_chars() {
        let content = r#"PATH!
/usr/bin

ENV!
VAR1 /path/to/file
VAR2 value=with=equals
VAR3 value:with:colons
"#;
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.env_vars.len(), 3);
        assert_eq!(
            parsed.env_vars[0],
            ("VAR1".to_string(), "/path/to/file".to_string())
        );
        assert_eq!(
            parsed.env_vars[1],
            ("VAR2".to_string(), "value=with=equals".to_string())
        );
        assert_eq!(
            parsed.env_vars[2],
            ("VAR3".to_string(), "value:with:colons".to_string())
        );
    }
}
