use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::atomic_file::AtomicFile;

#[derive(Debug, Clone, Default)]
pub struct Config {
    pub search: SearchConfig,
}

#[derive(Debug, Clone)]
pub struct SearchConfig {
    pub executable_search_fuzzy: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            executable_search_fuzzy: false,
        }
    }
}

/// Get the config file path
pub fn get_config_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".whi").join("config.toml"))
}

/// Load config from file, or return default if file doesn't exist
pub fn load_config() -> Result<Config, String> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        return Ok(Config::default());
    }

    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config file: {e}"))?;

    parse_config(&content)
}

/// Create default config file if it doesn't exist
pub fn ensure_config_exists() -> Result<(), String> {
    let config_path = get_config_path()?;

    if config_path.exists() {
        return Ok(());
    }

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create .whi directory: {e}"))?;
    }

    let default_config = generate_default_config();
    let mut atomic_file =
        AtomicFile::new(&config_path).map_err(|e| format!("Failed to create config file: {e}"))?;

    atomic_file
        .write_all(default_config.as_bytes())
        .map_err(|e| format!("Failed to write config: {e}"))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit config file: {e}"))?;

    Ok(())
}

fn generate_default_config() -> String {
    let defaults = Config::default();

    format!(
        "# whi configuration file\n# This file is automatically created with default values\n\n[search]\n# Enable fuzzy search for executables (default: {exec_fuzzy})\n# When enabled: 'whi cargo' finds cargo, cargo-clippy, cargo-fmt, etc.\n# When disabled: 'whi cargo' finds only exact match 'cargo'\nexecutable_search_fuzzy = {exec_fuzzy}\n\n# NOTE: Protected paths configuration lives in ~/.whi/protected_paths\n",
        exec_fuzzy = defaults.search.executable_search_fuzzy,
    )
}

fn parse_config(content: &str) -> Result<Config, String> {
    let mut config = Config::default();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            if current_section.as_str() == "search" && key == "executable_search_fuzzy" {
                config.search.executable_search_fuzzy = parse_bool(value)?;
            }
        }
    }

    Ok(config)
}

fn parse_bool(s: &str) -> Result<bool, String> {
    match s.to_lowercase().as_str() {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("Invalid boolean value: {s}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(!config.search.executable_search_fuzzy);
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[search]
executable_search_fuzzy = true
"#;

        let config = parse_config(toml).unwrap();
        assert!(config.search.executable_search_fuzzy);
    }

    #[test]
    fn test_parse_config_ignores_old_sections() {
        let toml = r#"
[search]
executable_search_fuzzy = true

[protected]
paths = ["/bin"]
"#;

        let config = parse_config(toml).unwrap();
        assert!(config.search.executable_search_fuzzy);
    }

    #[test]
    fn test_generate_default_config() {
        let default_toml = generate_default_config();
        let config = parse_config(&default_toml).unwrap();
        assert!(!config.search.executable_search_fuzzy);
    }
}
