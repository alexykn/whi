use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::io::atomic_file::AtomicFile;

fn default_protected_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        vec![
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/local/sbin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/sbin"),
        ]
    }

    #[cfg(target_os = "linux")]
    {
        vec![
            PathBuf::from("/usr/local/sbin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/bin"),
        ]
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")]
    }
}

pub fn get_protected_paths_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".whi").join("protected_paths"))
}

fn get_migration_marker_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".whi").join(".migrated"))
}

fn parse_protected_paths(content: &str) -> Result<Vec<PathBuf>, String> {
    use crate::io::line_utils::strip_inline_comment;

    let mut paths = Vec::new();
    let mut found_header = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let without_comment = strip_inline_comment(trimmed);
        if without_comment.is_empty() {
            continue;
        }

        if without_comment == "!protected.paths" {
            found_header = true;
            continue;
        }

        if found_header {
            paths.push(PathBuf::from(without_comment));
        }
    }

    if !found_header {
        return Err("Missing !protected.paths header".to_string());
    }

    Ok(paths)
}

fn format_protected_paths(paths: &[PathBuf]) -> String {
    let mut result = String::from("!protected.paths\n");
    for path in paths {
        result.push_str(&path.to_string_lossy());
        result.push('\n');
    }
    result
}

pub fn load_protected_paths() -> Result<Vec<PathBuf>, String> {
    let path = get_protected_paths_path()?;

    if !path.exists() {
        ensure_protected_paths_exists()?;
        return Ok(default_protected_paths());
    }

    let content = fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read {} file: {e}", path.display()))?;
    parse_protected_paths(&content)
}

pub fn ensure_protected_paths_exists() -> Result<(), String> {
    let path = get_protected_paths_path()?;
    if path.exists() {
        return Ok(());
    }

    save_protected_paths(&default_protected_paths())
}

pub fn save_protected_paths(paths: &[PathBuf]) -> Result<(), String> {
    let path = get_protected_paths_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create .whi directory: {e}"))?;
    }

    let content = format_protected_paths(paths);
    let mut atomic_file = AtomicFile::new(&path)
        .map_err(|e| format!("Failed to create {} file: {e}", path.display()))?;

    atomic_file
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit {} file: {e}", path.display()))?;

    Ok(())
}

pub fn migrate_from_config_toml() -> Result<bool, String> {
    use std::io::Write;

    let marker_path = get_migration_marker_path()?;
    if marker_path.exists() {
        return Ok(false);
    }

    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    let config_path = PathBuf::from(&home).join(".whi").join("config.toml");
    let protected_paths_file = get_protected_paths_path()?;

    if protected_paths_file.exists() {
        write_migration_marker(&marker_path)?;
        return Ok(false);
    }

    if !config_path.exists() {
        write_migration_marker(&marker_path)?;
        return Ok(false);
    }

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config.toml: {e}"))?;

    if !content.contains("[protected]") {
        write_migration_marker(&marker_path)?;
        return Ok(false);
    }

    let migrated_paths = parse_protected_paths_from_toml(&content);
    if !migrated_paths.is_empty() {
        save_protected_paths(&migrated_paths)?;
    }

    let rewritten = remove_protected_section(&content);
    let mut atomic_file =
        AtomicFile::new(&config_path).map_err(|e| format!("Failed to create config file: {e}"))?;
    atomic_file
        .write_all(rewritten.as_bytes())
        .map_err(|e| format!("Failed to write config.toml: {e}"))?;
    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit config.toml: {e}"))?;

    write_migration_marker(&marker_path)?;
    Ok(!migrated_paths.is_empty())
}

fn parse_protected_paths_from_toml(content: &str) -> Vec<PathBuf> {
    let mut in_protected = false;
    let mut paths = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_protected = trimmed == "[protected]";
            continue;
        }

        if !in_protected || !trimmed.starts_with("paths") {
            continue;
        }

        if let Some((_, values)) = trimmed.split_once('=') {
            for entry in values.split('"').skip(1).step_by(2) {
                if !entry.is_empty() {
                    paths.push(PathBuf::from(entry));
                }
            }
        }
    }

    paths
}

fn remove_protected_section(content: &str) -> String {
    let mut result = Vec::new();
    let mut in_protected = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_protected = trimmed == "[protected]";
            if in_protected {
                continue;
            }
        }

        if !in_protected {
            result.push(line);
        }
    }

    let mut rewritten = result.join("\n");
    if !rewritten.is_empty() {
        rewritten.push('\n');
    }
    rewritten.push_str(
        "# MIGRATION NOTE: The [protected] section has been migrated to ~/.whi/protected_paths\n",
    );
    rewritten
}

fn write_migration_marker(marker_path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create .whi directory: {e}"))?;
    }

    let mut atomic_file = AtomicFile::new(marker_path)
        .map_err(|e| format!("Failed to create migration marker: {e}"))?;
    atomic_file
        .write_all(b"# protected_paths migration completed\n")
        .map_err(|e| format!("Failed to write migration marker: {e}"))?;
    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit migration marker: {e}"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_protected_paths() {
        let content = "!protected.paths\n/usr/bin\n/bin\n";
        let parsed = parse_protected_paths(content).unwrap();
        assert_eq!(
            parsed,
            vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")]
        );
    }

    #[test]
    fn test_format_protected_paths() {
        let content = format_protected_paths(&[PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
        assert!(content.contains("!protected.paths"));
        assert!(content.contains("/usr/bin"));
        assert!(content.contains("/bin"));
    }

    #[test]
    fn test_parse_protected_paths_from_toml() {
        let content = r#"
[protected]
paths = ["/usr/bin", "/bin"]
"#;

        let parsed = parse_protected_paths_from_toml(content);
        assert_eq!(
            parsed,
            vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")]
        );
    }
}
