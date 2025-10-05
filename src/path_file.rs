/// Human-friendly `PATH` file format utilities
///
/// Format v2 (0.6.0+):
/// ```text
/// !path.replace
/// /usr/bin
/// /bin
///
/// !env.set
/// VAR value
/// ```
///
/// # Environment Variable Operations (Order-Dependent)
///
/// Environment operations (`!env.replace`, `!env.set`, `!env.unset`) are executed
/// **in the order they appear** in the whifile. This allows flexible patterns:
///
/// **Pattern 1: Replace then override**
/// ```text
/// !env.replace
/// MIN_VAR minimal_value
///
/// !env.set
/// EXTRA_VAR additional_value
/// ```
/// This first replaces the entire environment (unsetting all non-protected vars),
/// then sets `EXTRA_VAR` on top of the minimal environment.
///
/// **Pattern 2: Set then unset**
/// ```text
/// !env.set
/// DEBUG 1
///
/// !env.unset
/// PRODUCTION_KEY
/// ```
/// Sets `DEBUG`, then explicitly unsets `PRODUCTION_KEY`.
///
/// **Important:** `!env.replace` only protects variables listed in `~/.whi/protected_vars`.
/// To unset a protected variable, use explicit `!env.unset` (use with caution!).
///
/// Legacy format (pre-0.6.0):
/// ```text
/// PATH!
/// /usr/bin
/// /bin
///
/// ENV!
/// VAR value
/// ```
/// `PATH` section configuration for whifile
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PathSections {
    /// Replace session `PATH` entirely (mutually exclusive with prepend/append)
    pub replace: Option<Vec<String>>,
    /// Prepend to session `PATH`
    pub prepend: Vec<String>,
    /// Append to session `PATH`
    pub append: Vec<String>,
}

/// Individual environment variable operation
#[derive(Debug, Clone, PartialEq)]
pub enum EnvOperation {
    /// Replace entire environment (unsets all non-protected vars, then sets these)
    Replace(Vec<(String, String)>),
    /// Set a single environment variable
    Set(String, String),
    /// Unset a single environment variable
    Unset(String),
}

/// `ENV` section configuration for whifile
/// Operations are executed in the order they appear in the file
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EnvSections {
    /// Ordered list of environment operations
    pub operations: Vec<EnvOperation>,
}

/// Parsed path file containing both `PATH` and `ENV` vars
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPathFile {
    pub path: PathSections,
    pub env: EnvSections,
}

/// Format a `PATH` string into the human-friendly file format (v2 format)
#[must_use]
pub fn format_path_file(path: &str) -> String {
    format_path_file_with_env(path, &[])
}

/// Format a `PATH` string with optional `ENV` vars into the human-friendly file format (v2 format)
#[must_use]
pub fn format_path_file_with_env(path: &str, env_vars: &[(String, String)]) -> String {
    let mut output = String::from("!path.replace\n");

    for entry in path.split(':').filter(|s| !s.is_empty()) {
        output.push_str(entry);
        output.push('\n');
    }

    // Add ENV section
    output.push_str("\n!env.set\n");
    for (key, value) in env_vars {
        output.push_str(key);
        output.push(' ');
        output.push_str(value);
        output.push('\n');
    }

    output
}

/// Generate default whifile template with commented sections and protected paths
#[must_use]
pub fn default_whifile_template(protected_paths: &[String]) -> String {
    let mut output = String::new();

    // PATH section header with exclusivity rules
    output.push_str("# PATH directives (choose ONE approach):\n");
    output.push_str("#\n");
    output.push_str("# !path.replace - Replace entire session PATH\n");
    output.push_str("#   (exclusive: cannot be used with !path.append or !path.prepend)\n");
    output.push_str("#   Protected paths are included by default to prevent system breakage\n");
    output.push_str("#\n");
    output.push_str("!path.replace\n");

    // Add protected paths
    for path in protected_paths {
        output.push_str(path);
        output.push('\n');
    }
    output.push('\n');

    // Alternative: prepend/append (commented out)
    output.push_str("# !path.prepend - Add paths to beginning of session PATH\n");
    output.push_str("#   (can be combined with !path.append)\n");
    output.push_str("#\n");
    output.push_str("# !path.prepend\n");
    output.push_str("# /my/custom/path\n\n");

    output.push_str("# !path.append - Add paths to end of session PATH\n");
    output.push_str("#   (can be combined with !path.prepend)\n");
    output.push_str("#\n");
    output.push_str("# !path.append\n");
    output.push_str("# /another/path\n\n");

    // ENV section header with order-dependent behavior
    output.push_str("# ENV directives (IMPORTANT: executed in the order they appear!):\n");
    output.push_str("#\n");
    output.push_str("# Operations are processed sequentially, allowing powerful patterns:\n");
    output.push_str("# 1. !env.replace -> !env.set   (minimal env + extras)\n");
    output.push_str("# 2. !env.set -> !env.unset     (set then selectively remove)\n");
    output.push_str("# 3. Multiple sections of same type work as expected\n");
    output.push_str("#\n");
    output.push_str("# !env.set - Set environment variables\n");
    output.push_str("#\n");
    output.push_str("!env.set\n");
    output.push_str("# KEY value\n\n");

    output.push_str("# !env.unset - Unset environment variables\n");
    output.push_str("#   Works on any variable, including those set above\n");
    output.push_str("#\n");
    output.push_str("# !env.unset\n");
    output.push_str("# VAR_TO_REMOVE\n\n");

    output.push_str("# !env.replace - Replace entire environment\n");
    output.push_str("#   Unsets ALL non-protected vars, then sets only the ones listed below\n");
    output.push_str("#   Protected vars (see ~/.whi/protected_vars) are NEVER unset by replace\n");
    output.push_str(
        "#   To unset protected vars, use explicit !env.unset AFTER replace (careful!)\n",
    );
    output.push_str("#\n");
    output.push_str("# !env.replace\n");
    output.push_str("# KEY value\n");
    output.push_str("# KEY2 value2\n\n");

    output.push_str("# Common pattern: Minimal environment + selective additions\n");
    output.push_str("# !env.replace\n");
    output.push_str("# MINIMAL_VAR value    # Start with minimal env\n");
    output.push_str("#\n");
    output.push_str("# !env.set             # Then add extras (executed AFTER replace)\n");
    output.push_str("# EXTRA_VAR additional_value\n");

    output
}

/// Parse `PATH` file - supports v2 (!path.replace), v1 (PATH!), and legacy (colon-separated) formats
///
/// Returns `ParsedPathFile` with path sections and env sections.
/// This provides backward compatibility with `saved_path` and profile files from all previous releases.
pub fn parse_path_file(content: &str) -> Result<ParsedPathFile, String> {
    let trimmed = content.trim();

    if trimmed.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    // Find first non-comment, non-empty line to detect format
    let first_directive = trimmed
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or("");

    // Check if this is v2 format (starts with !)
    if first_directive.starts_with('!') {
        parse_v2_format(trimmed)
    } else if first_directive.starts_with("PATH!") {
        // v1 format (0.5.x): PATH!\n/path1\n/path2\n\nENV!\n - convert to v2
        parse_v1_format(trimmed)
    } else {
        // Legacy format (pre-0.5.0): colon-separated string - convert to v2
        parse_legacy_format(trimmed)
    }
}

/// Process PATH section line
fn process_path_line(section: &str, line: &str, path_sections: &mut PathSections) {
    match section {
        "replace" => {
            path_sections
                .replace
                .get_or_insert_with(Vec::new)
                .push(line.to_string());
        }
        "prepend" => {
            path_sections.prepend.push(line.to_string());
        }
        "append" => {
            path_sections.append.push(line.to_string());
        }
        _ => {}
    }
}

/// Process ENV section line
fn process_env_line(
    section: &str,
    line: &str,
    env_sections: &mut EnvSections,
    env_replace_buffer: &mut Vec<(String, String)>,
) -> Result<(), String> {
    match section {
        "replace" => {
            parse_env_line(line, env_replace_buffer)?;
        }
        "set" => {
            let mut temp = Vec::new();
            parse_env_line(line, &mut temp)?;
            for (key, value) in temp {
                env_sections.operations.push(EnvOperation::Set(key, value));
            }
        }
        "unset" => {
            env_sections
                .operations
                .push(EnvOperation::Unset(line.to_string()));
        }
        _ => {}
    }
    Ok(())
}

fn parse_v2_format(content: &str) -> Result<ParsedPathFile, String> {
    use crate::file_utils::strip_inline_comment;

    let mut path_sections = PathSections::default();
    let mut env_sections = EnvSections::default();

    let mut current_path_section: Option<&str> = None;
    let mut current_env_section: Option<&str> = None;
    let mut env_replace_buffer: Vec<(String, String)> = Vec::new();

    let flush_replace = |env_sections: &mut EnvSections,
                         env_replace_buffer: &mut Vec<(String, String)>| {
        if !env_replace_buffer.is_empty() {
            env_sections
                .operations
                .push(EnvOperation::Replace(env_replace_buffer.clone()));
            env_replace_buffer.clear();
        }
    };

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Strip inline comments
        let line = strip_inline_comment(line);

        // Skip if line becomes empty after stripping comment
        if line.is_empty() {
            continue;
        }

        // Check for section headers
        match line {
            "!path.replace" | "!path.saved" => {
                flush_replace(&mut env_sections, &mut env_replace_buffer);
                current_path_section = Some("replace");
                current_env_section = None;
                continue;
            }
            "!path.prepend" => {
                flush_replace(&mut env_sections, &mut env_replace_buffer);
                current_path_section = Some("prepend");
                current_env_section = None;
                continue;
            }
            "!path.append" => {
                flush_replace(&mut env_sections, &mut env_replace_buffer);
                current_path_section = Some("append");
                current_env_section = None;
                continue;
            }
            "!env.replace" => {
                flush_replace(&mut env_sections, &mut env_replace_buffer);
                current_path_section = None;
                current_env_section = Some("replace");
                continue;
            }
            "!env.set" | "!env.saved" => {
                flush_replace(&mut env_sections, &mut env_replace_buffer);
                current_path_section = None;
                current_env_section = Some("set");
                continue;
            }
            "!env.unset" => {
                flush_replace(&mut env_sections, &mut env_replace_buffer);
                current_path_section = None;
                current_env_section = Some("unset");
                continue;
            }
            _ => {}
        }

        // Process content based on current section
        if let Some(section) = current_path_section {
            process_path_line(section, line, &mut path_sections);
        } else if let Some(section) = current_env_section {
            process_env_line(section, line, &mut env_sections, &mut env_replace_buffer)?;
        }
    }

    flush_replace(&mut env_sections, &mut env_replace_buffer);

    if path_sections.replace.is_some()
        && (!path_sections.prepend.is_empty() || !path_sections.append.is_empty())
    {
        return Err("Cannot combine !path.replace with !path.prepend or !path.append".to_string());
    }

    if path_sections.replace.is_none()
        && path_sections.prepend.is_empty()
        && path_sections.append.is_empty()
    {
        return Err(
            "No PATH entries found in file (template needs to be edited with actual paths)"
                .to_string(),
        );
    }

    Ok(ParsedPathFile {
        path: path_sections,
        env: env_sections,
    })
}

/// Validate environment variable name
fn is_valid_env_name(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }

    let mut chars = name.chars();
    // First character must be letter or underscore
    if let Some(first) = chars.next() {
        if !first.is_ascii_alphabetic() && first != '_' {
            return false;
        }
    }

    // Rest must be alphanumeric or underscore
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse `ENV` var line: `KEY` value (space-separated, fish-style)
/// Returns error if `KEY` contains invalid characters
fn parse_env_line(line: &str, env_list: &mut Vec<(String, String)>) -> Result<(), String> {
    let (key, value) = if let Some(space_idx) = line.find(char::is_whitespace) {
        let key = line[..space_idx].to_string();
        let value = line[space_idx..].trim().to_string();
        (key, value)
    } else {
        // No value, set to empty string
        (line.to_string(), String::new())
    };

    // Validate env var name: must start with letter/underscore, contain only alphanumeric/underscore
    if !is_valid_env_name(&key) {
        // Check for common mistakes and give helpful suggestions
        let suggestion = if key.contains('=') {
            let parts: Vec<&str> = key.splitn(2, '=').collect();
            if parts.len() == 2 {
                format!(". Try '{} {}' instead", parts[0], parts[1])
            } else {
                String::new()
            }
        } else if key.contains('-') {
            ". Hyphens are not allowed, use underscores instead".to_string()
        } else if key.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            ". Variable names cannot start with a number".to_string()
        } else {
            String::new()
        };

        return Err(format!(
            "Invalid environment variable name: '{key}'{suggestion}"
        ));
    }

    env_list.push((key, value));
    Ok(())
}

fn parse_v1_format(content: &str) -> Result<ParsedPathFile, String> {
    use crate::file_utils::strip_inline_comment;

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

        // Strip inline comments
        let line = strip_inline_comment(line);

        // Skip if line becomes empty after stripping comment
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
            parse_env_line(line, &mut env_vars)?;
        }
    }

    if path_entries.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    // Convert to v2 format: PATH! becomes !path.replace, ENV! becomes !env.set operations
    let mut operations = Vec::new();
    for (key, value) in env_vars {
        operations.push(EnvOperation::Set(key, value));
    }

    Ok(ParsedPathFile {
        path: PathSections {
            replace: Some(path_entries),
            prepend: Vec::new(),
            append: Vec::new(),
        },
        env: EnvSections { operations },
    })
}

/// Apply `PATH` sections to a base `PATH`
///
/// - If `replace` is provided, use it as the new `PATH` (ignoring `base_path`)
/// - Otherwise, start with `base_path` and apply prepend/append
///
/// Returns colon-separated `PATH` string with duplicates removed
pub fn apply_path_sections(base_path: &str, sections: &PathSections) -> Result<String, String> {
    let mut entries: Vec<String> = Vec::new();

    if let Some(replace_entries) = &sections.replace {
        // Replace mode: use only these entries
        entries.clone_from(replace_entries);
    } else {
        // Prepend/append mode: start with base PATH
        let base_entries: Vec<String> = base_path
            .split(':')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        // Prepend entries
        entries.extend(sections.prepend.iter().cloned());

        // Add base entries
        entries.extend(base_entries);

        // Append entries
        entries.extend(sections.append.iter().cloned());
    }

    // Deduplicate while preserving order
    let mut seen = std::collections::HashSet::new();
    let unique_entries: Vec<String> = entries
        .into_iter()
        .filter(|e| !e.is_empty() && seen.insert(e.clone()))
        .collect();

    if unique_entries.is_empty() {
        return Err("Resulting PATH is empty".to_string());
    }

    Ok(unique_entries.join(":"))
}

/// Parse legacy colon-separated format and convert to v2
fn parse_legacy_format(content: &str) -> Result<ParsedPathFile, String> {
    // Join all lines and split by colon to handle both single and multi-line legacy files
    let all_lines = content.lines().map(str::trim).collect::<Vec<_>>().join("");

    let entries: Vec<String> = all_lines
        .split(':')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    if entries.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    // Convert to v2 format: legacy becomes !path.replace
    Ok(ParsedPathFile {
        path: PathSections {
            replace: Some(entries),
            prepend: Vec::new(),
            append: Vec::new(),
        },
        env: EnvSections::default(), // Legacy format has no ENV vars
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_path_file() {
        let path = "/usr/bin:/bin:/usr/local/bin";
        let formatted = format_path_file(path);

        assert!(formatted.contains("!path.replace"));
        assert!(formatted.contains("/usr/bin\n"));
        assert!(formatted.contains("/bin\n"));
        assert!(formatted.contains("/usr/local/bin\n"));
        assert!(formatted.contains("!env.set"));
    }

    #[test]
    fn test_parse_v1_format() {
        // Test v1 format (PATH!/ENV!) - should convert to v2
        let content = r#"PATH!
/usr/bin
/bin
/usr/local/bin

ENV!
"#;
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(
            parsed.path.replace.as_ref().unwrap().join(":"),
            "/usr/bin:/bin:/usr/local/bin"
        );
        assert!(parsed.env.operations.is_empty());
    }

    #[test]
    fn test_roundtrip() {
        let original = "/usr/bin:/bin:/usr/local/bin:/opt/bin";
        let formatted = format_path_file(original);
        let parsed = parse_path_file(&formatted).unwrap();
        let reconstructed = apply_path_sections("", &parsed.path).unwrap();
        assert_eq!(reconstructed, original);
        assert!(parsed.env.operations.is_empty());
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
        assert_eq!(
            parsed.path.replace.as_ref().unwrap().join(":"),
            "/usr/bin:/bin:/usr/local/bin"
        );
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
    fn test_parse_legacy_format() {
        let content = "/usr/bin:/bin:/usr/local/bin";
        let parsed = parse_path_file(content).unwrap();
        let result = apply_path_sections("", &parsed.path).unwrap();
        assert_eq!(result, "/usr/bin:/bin:/usr/local/bin");
        assert!(parsed.env.operations.is_empty());
    }

    #[test]
    fn test_parse_v2_replace() {
        let content = r#"!path.replace
/usr/bin
/bin

!env.set
"#;
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(
            parsed.path.replace.as_ref().unwrap().join(":"),
            "/usr/bin:/bin"
        );
        assert!(parsed.path.prepend.is_empty());
        assert!(parsed.path.append.is_empty());
    }

    #[test]
    fn test_parse_v2_prepend_append() {
        let content = r#"!path.prepend
/opt/local/bin

!path.append
/usr/local/bin

!env.set
"#;
        let parsed = parse_path_file(content).unwrap();
        assert!(parsed.path.replace.is_none());
        assert_eq!(parsed.path.prepend, vec!["/opt/local/bin"]);
        assert_eq!(parsed.path.append, vec!["/usr/local/bin"]);
    }

    #[test]
    fn test_parse_v2_mutual_exclusivity_error() {
        let content = r#"!path.replace
/usr/bin

!path.prepend
/opt/bin
"#;
        let result = parse_path_file(content);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Cannot combine"));
    }

    #[test]
    fn test_parse_v2_env_sections() {
        let content = r#"!path.replace
/usr/bin

!env.set
VAR1 value1
VAR2 value2

!env.unset
OLD_VAR
"#;
        let parsed = parse_path_file(content).unwrap();
        // Should have 2 Set operations and 1 Unset operation
        assert_eq!(parsed.env.operations.len(), 3);
        assert!(
            matches!(&parsed.env.operations[0], EnvOperation::Set(k, v) if k == "VAR1" && v == "value1")
        );
        assert!(
            matches!(&parsed.env.operations[1], EnvOperation::Set(k, v) if k == "VAR2" && v == "value2")
        );
        assert!(matches!(&parsed.env.operations[2], EnvOperation::Unset(k) if k == "OLD_VAR"));
    }

    #[test]
    fn test_apply_path_sections_replace() {
        let sections = PathSections {
            replace: Some(vec!["/usr/bin".to_string(), "/bin".to_string()]),
            prepend: Vec::new(),
            append: Vec::new(),
        };
        let result = apply_path_sections("/old/path", &sections).unwrap();
        assert_eq!(result, "/usr/bin:/bin");
    }

    #[test]
    fn test_apply_path_sections_prepend_append() {
        let sections = PathSections {
            replace: None,
            prepend: vec!["/opt/bin".to_string()],
            append: vec!["/usr/local/bin".to_string()],
        };
        let result = apply_path_sections("/usr/bin:/bin", &sections).unwrap();
        assert_eq!(result, "/opt/bin:/usr/bin:/bin:/usr/local/bin");
    }

    #[test]
    fn test_apply_path_sections_dedup() {
        let sections = PathSections {
            replace: None,
            prepend: vec!["/usr/bin".to_string()],
            append: vec!["/bin".to_string()],
        };
        let result = apply_path_sections("/usr/bin:/bin", &sections).unwrap();
        // Should deduplicate, keeping first occurrence
        assert_eq!(result, "/usr/bin:/bin");
    }

    #[test]
    fn test_default_template() {
        let protected_paths = vec!["/usr/bin".to_string(), "/bin".to_string()];
        let template = default_whifile_template(&protected_paths);
        assert!(template.contains("!path.replace\n"));
        assert!(template.contains("/usr/bin\n"));
        assert!(template.contains("/bin\n"));
        assert!(template.contains("# !path.prepend\n"));
        assert!(template.contains("# !path.append\n"));
        assert!(template.contains("!env.set\n"));
        assert!(template.contains("# !env.replace\n"));
        assert!(template.contains("# !env.unset\n"));
    }

    #[test]
    fn test_backward_compat_v1_to_v2() {
        let v1_content = r#"PATH!
/usr/bin
/bin

ENV!
VAR value
"#;
        let parsed = parse_path_file(v1_content).unwrap();
        // v1 PATH! should convert to !path.replace
        assert!(parsed.path.replace.is_some());
        assert_eq!(
            parsed.path.replace.as_ref().unwrap().join(":"),
            "/usr/bin:/bin"
        );
        // v1 ENV! should convert to !env.set operations
        assert_eq!(parsed.env.operations.len(), 1);
        assert!(
            matches!(&parsed.env.operations[0], EnvOperation::Set(k, v) if k == "VAR" && v == "value")
        );
    }

    #[test]
    fn test_backward_compat_legacy_to_v2() {
        let legacy = "/usr/bin:/bin:/usr/local/bin";
        let parsed = parse_path_file(legacy).unwrap();
        // Legacy should convert to !path.replace
        assert!(parsed.path.replace.is_some());
        let result = apply_path_sections("", &parsed.path).unwrap();
        assert_eq!(result, legacy);
    }

    #[test]
    fn test_invalid_env_var_name_number_start() {
        let content = "!path.replace\n/usr/bin\n\n!env.set\n2INVALID value";
        let result = parse_path_file(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid environment variable name"));
        assert!(err.contains("2INVALID"));
        assert!(err.contains("Variable names cannot start with a number"));
    }

    #[test]
    fn test_invalid_env_var_name_special_chars() {
        let content = "!path.replace\n/usr/bin\n\n!env.set\nINVALID-NAME value";
        let result = parse_path_file(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid environment variable name"));
        assert!(err.contains("Hyphens are not allowed, use underscores instead"));
    }

    #[test]
    fn test_invalid_env_var_name_equals() {
        let content = "!path.replace\n/usr/bin\n\n!env.set\nSPS2_ALLOW_HTTP=1";
        let result = parse_path_file(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.contains("Invalid environment variable name"));
        assert!(err.contains("Try 'SPS2_ALLOW_HTTP 1' instead"));
    }

    #[test]
    fn test_valid_env_var_names() {
        let content = "!path.replace\n/usr/bin\n\n!env.set\nVAR1 value\n_VAR2 value\nVAR_3 value";
        let parsed = parse_path_file(content).unwrap();
        assert_eq!(parsed.env.operations.len(), 3);
        assert!(matches!(&parsed.env.operations[0], EnvOperation::Set(k, _) if k == "VAR1"));
        assert!(matches!(&parsed.env.operations[1], EnvOperation::Set(k, _) if k == "_VAR2"));
        assert!(matches!(&parsed.env.operations[2], EnvOperation::Set(k, _) if k == "VAR_3"));
    }

    #[test]
    fn test_inline_comments_in_whifiles() {
        let content = r"# PATH directives
!path.replace
/usr/local/bin
/usr/bin
/bin
/usr/sbin     # inline comment
/sbin # inline comment
/Users/$USER/.cargo/bin

!env.set
TEST_VAR1 $(pwd)     # command substitution with comment
TEST_VAR2 $HOME # variable with comment
TEST_VAR3 TEST#comment without space
";

        let parsed = parse_path_file(content).unwrap();

        // Verify paths parsed correctly (inline comments stripped)
        let paths = parsed.path.replace.as_ref().unwrap();
        assert_eq!(paths.len(), 6);
        assert_eq!(paths[0], "/usr/local/bin");
        assert_eq!(paths[1], "/usr/bin");
        assert_eq!(paths[2], "/bin");
        assert_eq!(paths[3], "/usr/sbin");
        assert_eq!(paths[4], "/sbin");
        assert_eq!(paths[5], "/Users/$USER/.cargo/bin");

        // Verify env vars parsed correctly
        assert_eq!(parsed.env.operations.len(), 3);
        assert!(
            matches!(&parsed.env.operations[0], EnvOperation::Set(k, v) if k == "TEST_VAR1" && v == "$(pwd)")
        );
        assert!(
            matches!(&parsed.env.operations[1], EnvOperation::Set(k, v) if k == "TEST_VAR2" && v == "$HOME")
        );
        assert!(
            matches!(&parsed.env.operations[2], EnvOperation::Set(k, v) if k == "TEST_VAR3" && v == "TEST")
        );
    }

    #[test]
    fn test_inline_comments_v1_format() {
        // Test that inline comments work with legacy v1 format too
        let content = r"PATH!
/usr/bin     # system binaries
/bin # more binaries

ENV!
MY_VAR value # some value
";

        let parsed = parse_path_file(content).unwrap();

        let paths = parsed.path.replace.as_ref().unwrap();
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], "/usr/bin");
        assert_eq!(paths[1], "/bin");

        assert_eq!(parsed.env.operations.len(), 1);
        assert!(
            matches!(&parsed.env.operations[0], EnvOperation::Set(k, v) if k == "MY_VAR" && v == "value")
        );
    }
}
