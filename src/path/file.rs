use std::collections::BTreeSet;
use std::env;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PathSections {
    pub replace: Option<Vec<String>>,
    pub prepend: Vec<String>,
    pub append: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedPathFile {
    pub path: PathSections,
}

#[must_use]
pub fn format_path_file(path: &str) -> String {
    let mut output = String::from("!path.replace\n");

    for entry in path.split(':').filter(|s| !s.is_empty()) {
        output.push_str(entry);
        output.push('\n');
    }

    output
}

pub fn parse_path_file(content: &str) -> Result<ParsedPathFile, String> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    let first_directive = trimmed
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or("");

    if first_directive.starts_with('!') {
        parse_v2_format(trimmed)
    } else if first_directive.starts_with("PATH!") || first_directive.starts_with("ENV!") {
        parse_v1_format(trimmed)
    } else {
        parse_legacy_format(trimmed)
    }
}

pub fn apply_path_sections(base_path: &str, sections: &PathSections) -> Result<String, String> {
    let mut entries: Vec<String> = Vec::new();

    if let Some(replace_entries) = &sections.replace {
        entries.clone_from(replace_entries);
    } else {
        let base_entries: Vec<String> = base_path
            .split(':')
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        entries.extend(sections.prepend.iter().cloned());
        entries.extend(base_entries);
        entries.extend(sections.append.iter().cloned());
    }

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

#[must_use]
pub fn expand_shell_vars(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    let mut at_start = true;

    while let Some(ch) = chars.next() {
        if ch == '~' && (at_start || result.ends_with(':') || result.ends_with(' ')) {
            if chars.peek() == Some(&'/') || chars.peek().is_none() || chars.peek() == Some(&':') {
                if let Ok(home) = env::var("HOME") {
                    result.push_str(&home);
                } else {
                    result.push('~');
                }
            } else {
                result.push('~');
            }
            at_start = false;
            continue;
        }

        if ch == '$' {
            if chars.peek() == Some(&'{') {
                chars.next();
                let mut var_name = String::new();

                for c in chars.by_ref() {
                    if c == '}' {
                        break;
                    }
                    var_name.push(c);
                }

                if let Ok(val) = env::var(&var_name) {
                    result.push_str(&val);
                }
            } else {
                let mut var_name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        var_name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }

                if var_name.is_empty() {
                    result.push('$');
                } else if let Ok(val) = env::var(&var_name) {
                    result.push_str(&val);
                }
            }
            at_start = false;
            continue;
        }

        result.push(ch);
        at_start = false;
    }

    result
}

fn parse_v2_format(content: &str) -> Result<ParsedPathFile, String> {
    use crate::io::line_utils::strip_inline_comment;

    let mut path_sections = PathSections::default();
    let mut current_path_section: Option<&str> = None;
    let mut deprecated_directives = BTreeSet::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = strip_inline_comment(line);
        if line.is_empty() {
            continue;
        }

        match line {
            "!path.replace" | "!path.saved" => {
                current_path_section = Some("replace");
                continue;
            }
            "!path.prepend" => {
                current_path_section = Some("prepend");
                continue;
            }
            "!path.append" => {
                current_path_section = Some("append");
                continue;
            }
            "!env.replace" | "!env.set" | "!env.unset" | "!env.saved" => {
                deprecated_directives.insert("environment directives");
                current_path_section = None;
                continue;
            }
            "!whi.extra" => {
                deprecated_directives.insert("extra directives");
                current_path_section = None;
                continue;
            }
            _ => {}
        }

        if let Some(section) = current_path_section {
            process_path_line(section, line, &mut path_sections);
        }
    }

    validate_path_sections(&path_sections)?;
    warn_deprecated_directives(&deprecated_directives);

    Ok(ParsedPathFile {
        path: path_sections,
    })
}

fn process_path_line(section: &str, line: &str, path_sections: &mut PathSections) {
    match section {
        "replace" => {
            path_sections
                .replace
                .get_or_insert_with(Vec::new)
                .push(line.to_string());
        }
        "prepend" => path_sections.prepend.push(line.to_string()),
        "append" => path_sections.append.push(line.to_string()),
        _ => {}
    }
}

fn validate_path_sections(path_sections: &PathSections) -> Result<(), String> {
    if path_sections.replace.is_some()
        && (!path_sections.prepend.is_empty() || !path_sections.append.is_empty())
    {
        return Err("Cannot combine !path.replace with !path.prepend or !path.append".to_string());
    }

    let has_path = path_sections.replace.is_some()
        || !path_sections.prepend.is_empty()
        || !path_sections.append.is_empty();
    if !has_path {
        return Err("No PATH entries found in file".to_string());
    }

    Ok(())
}

fn parse_v1_format(content: &str) -> Result<ParsedPathFile, String> {
    use crate::io::line_utils::strip_inline_comment;

    let mut path_entries = Vec::new();
    let mut in_path_section = false;
    let mut saw_env_section = false;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let line = strip_inline_comment(line);
        if line.is_empty() {
            continue;
        }

        if line == "PATH!" {
            in_path_section = true;
            continue;
        }

        if line == "ENV!" {
            in_path_section = false;
            saw_env_section = true;
            continue;
        }

        if in_path_section {
            path_entries.push(line.to_string());
        }
    }

    if saw_env_section {
        let mut deprecated = BTreeSet::new();
        deprecated.insert("legacy ENV! sections");
        warn_deprecated_directives(&deprecated);
    }

    if path_entries.is_empty() {
        return Err("No PATH entries found in file".to_string());
    }

    Ok(ParsedPathFile {
        path: PathSections {
            replace: Some(path_entries),
            prepend: Vec::new(),
            append: Vec::new(),
        },
    })
}

fn parse_legacy_format(content: &str) -> Result<ParsedPathFile, String> {
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

    Ok(ParsedPathFile {
        path: PathSections {
            replace: Some(entries),
            prepend: Vec::new(),
            append: Vec::new(),
        },
    })
}

fn warn_deprecated_directives(directives: &BTreeSet<&str>) {
    if !directives.is_empty() {
        #[cfg(not(test))]
        eprintln!(
            "Warning: Ignoring deprecated {}. whi now manages PATH only.",
            directives.iter().copied().collect::<Vec<_>>().join(", ")
        );
    }
}
