use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::atomic_file::AtomicFile;

fn default_protected_paths() -> Vec<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        return vec![
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/sbin"),
        ];
    }

    #[cfg(target_os = "linux")]
    {
        return vec![
            PathBuf::from("/usr/local/sbin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/sbin"),
            PathBuf::from("/usr/bin"),
            PathBuf::from("/sbin"),
            PathBuf::from("/bin"),
        ];
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        return vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")];
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub venv: VenvConfig,
    pub protected: ProtectedConfig,
}

#[derive(Debug, Clone)]
pub struct VenvConfig {
    pub auto_activate_file: bool,
}

#[derive(Debug, Clone)]
pub struct ProtectedConfig {
    pub paths: Vec<PathBuf>,
}

impl Default for VenvConfig {
    fn default() -> Self {
        Self {
            auto_activate_file: false,
        }
    }
}

impl Default for ProtectedConfig {
    fn default() -> Self {
        Self {
            paths: default_protected_paths(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            venv: VenvConfig::default(),
            protected: ProtectedConfig::default(),
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

    // Create ~/.whi directory if needed
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

/// Generate default config TOML
fn generate_default_config() -> String {
    let defaults = Config::default();
    let path_entries = defaults
        .protected
        .paths
        .iter()
        .map(|p| format!("  \"{}\"", p.to_string_lossy()))
        .collect::<Vec<_>>()
        .join(",\n");

    format!(
        "# whi configuration file\n# This file is automatically created with default values\n\n[venv]\n# Auto-activate whi.file when entering directory (default: {auto_file})\nauto_activate_file = {auto_file}\n\n[protected]\n# Protected paths are preserved during 'whi apply' even if deleted in session\n# This prevents breaking your shell by accidentally saving a minimal PATH\npaths = [\n{paths}\n]\n",
        auto_file = defaults.venv.auto_activate_file,
        paths = path_entries
    )
}

/// Minimal TOML parser for our config
fn parse_config(content: &str) -> Result<Config, String> {
    let mut config = Config::default();
    let mut current_section = String::new();
    let mut in_array = false;
    let mut array_values: Vec<String> = Vec::new();
    let mut array_key = String::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Handle section headers
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        // Handle array start
        if line.contains('[') && !line.contains(']') {
            if let Some(key_part) = line.split('=').next() {
                array_key = key_part.trim().to_string();
                in_array = true;
                array_values.clear();
                continue;
            }
        }

        // Handle array values
        if in_array {
            if line.contains(']') {
                // End of array
                in_array = false;
                if current_section == "protected" && array_key == "paths" {
                    config.protected.paths = array_values
                        .iter()
                        .map(|s| PathBuf::from(s.trim_matches('"')))
                        .collect();
                }
                continue;
            } else if line.starts_with('"') || line.contains('"') {
                // Extract quoted value
                let value = line
                    .trim()
                    .trim_end_matches(',')
                    .trim_matches('"')
                    .to_string();
                if !value.is_empty() {
                    array_values.push(value);
                }
                continue;
            }
        }

        // Handle key-value pairs
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match current_section.as_str() {
                "venv" => match key {
                    "auto_activate_file" => {
                        config.venv.auto_activate_file = parse_bool(value)?;
                    }
                    _ => {} // Ignore unknown keys
                },
                _ => {} // Ignore unknown sections
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
        assert!(!config.venv.auto_activate_file);
        assert_eq!(config.protected.paths, default_protected_paths());
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[venv]
auto_activate_file = true

[protected]
paths = [
  "/bin",
  "/usr/bin",
]
"#;

        let config = parse_config(toml).unwrap();
        assert!(config.venv.auto_activate_file);
        assert_eq!(config.protected.paths.len(), 2);
        assert_eq!(config.protected.paths[0], PathBuf::from("/bin"));
    }

    #[test]
    fn test_generate_default_config() {
        let default_toml = generate_default_config();
        let config = parse_config(&default_toml).unwrap();
        assert!(!config.venv.auto_activate_file);
        assert_eq!(config.protected.paths, default_protected_paths());
    }
}
