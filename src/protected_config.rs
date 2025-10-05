use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::atomic_file::AtomicFile;

/// Trait for items that can be stored in protected configuration files
/// Implemented for both `String` (vars) and `PathBuf` (paths)
trait ProtectedItem: Sized {
    /// Parse an item from a line in the config file
    fn from_line(line: &str) -> Self;

    /// Convert item to string for file output
    fn to_file_string(&self) -> String;
}

impl ProtectedItem for String {
    fn from_line(line: &str) -> Self {
        line.to_string()
    }

    fn to_file_string(&self) -> String {
        self.clone()
    }
}

impl ProtectedItem for PathBuf {
    fn from_line(line: &str) -> Self {
        PathBuf::from(line)
    }

    fn to_file_string(&self) -> String {
        self.to_string_lossy().to_string()
    }
}

/// Generic parser for protected items (vars or paths)
fn parse_protected_items<T: ProtectedItem>(content: &str, header: &str) -> Result<Vec<T>, String> {
    use crate::file_utils::strip_inline_comment;

    let mut items = Vec::new();
    let mut found_header = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Strip inline comments
        let without_comment = strip_inline_comment(trimmed);

        // Skip if line becomes empty after stripping comment
        if without_comment.is_empty() {
            continue;
        }

        // Check for header
        if without_comment == header {
            found_header = true;
            continue;
        }

        // Only collect items after header is found
        if found_header {
            items.push(T::from_line(without_comment));
        }
    }

    if !found_header {
        return Err(format!("Missing {header} header"));
    }

    Ok(items)
}

/// Generic formatter for protected items (vars or paths)
fn format_protected_items<T: ProtectedItem>(items: &[T], header: &str) -> String {
    let mut result = String::from(header);
    result.push('\n');
    for item in items {
        result.push_str(&item.to_file_string());
        result.push('\n');
    }
    result
}

/// Generic loader for protected items (vars or paths)
fn load_protected_items<T: ProtectedItem>(
    path: &PathBuf,
    header: &str,
    defaults: Vec<T>,
    ensure_fn: fn() -> Result<(), String>,
    validate_fn: Option<fn(&[T])>,
) -> Result<Vec<T>, String> {
    if !path.exists() {
        // Create with defaults
        ensure_fn()?;
        return Ok(defaults);
    }

    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {} file: {e}", path.display()))?;

    let items = parse_protected_items(&content, header)?;

    // Run validation if provided
    if let Some(validate) = validate_fn {
        validate(&items);
    }

    Ok(items)
}

/// Generic saver for protected items (vars or paths)
fn save_protected_items<T: ProtectedItem>(
    items: &[T],
    path: &PathBuf,
    header: &str,
) -> Result<(), String> {
    // Create ~/.whi directory if needed
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create .whi directory: {e}"))?;
    }

    let content = format_protected_items(items, header);
    let mut atomic_file = AtomicFile::new(path)
        .map_err(|e| format!("Failed to create {} file: {e}", path.display()))?;

    atomic_file
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit {} file: {e}", path.display()))?;

    Ok(())
}

/// Generic ensure function for protected items (vars or paths)
fn ensure_protected_file_exists<T: ProtectedItem>(
    path: &PathBuf,
    header: &str,
    defaults: &[T],
) -> Result<(), String> {
    if path.exists() {
        return Ok(());
    }

    save_protected_items(defaults, path, header)
}

/// Default protected environment variables that should never be unset
fn default_protected_vars() -> Vec<String> {
    vec![
        // System critical - universal
        "PATH".to_string(),
        "HOME".to_string(),
        "USER".to_string(),
        "LOGNAME".to_string(),
        "SHELL".to_string(),
        "TERM".to_string(), // Terminal type - absolutely critical
        "TERMINFO".to_string(),
        "TERM_PROGRAM".to_string(),
        "TERM_PROGRAM_VERSION".to_string(),
        "LANG".to_string(),   // Locale - affects program behavior
        "LC_ALL".to_string(), // Locale overrides
        "LC_CTYPE".to_string(),
        "LC_MESSAGES".to_string(),
        "LC_NUMERIC".to_string(),
        "LC_COLLATE".to_string(),
        "LC_TIME".to_string(),
        "IFS".to_string(), // Internal field separator - DANGEROUS to unset
        // Shell state
        "PWD".to_string(),    // Current directory
        "OLDPWD".to_string(), // Previous directory
        "SHLVL".to_string(),  // Shell nesting level
        // Temp directories
        "TMPDIR".to_string(), // macOS/BSD standard temp dir
        "TMP".to_string(),    // Alternative temp dir
        "TEMP".to_string(),   // Windows-style temp dir
        // Display/GUI (X11/Wayland)
        "DISPLAY".to_string(),                  // X11 display
        "WAYLAND_DISPLAY".to_string(),          // Wayland display
        "XDG_RUNTIME_DIR".to_string(),          // XDG runtime directory
        "XDG_SESSION_TYPE".to_string(),         // Session type (x11/wayland)
        "XDG_DATA_DIRS".to_string(),            // XDG data directories
        "XAUTHORITY".to_string(),               // X11 auth cookie
        "DBUS_SESSION_BUS_ADDRESS".to_string(), // DBus session bus
        // SSH
        "SSH_AUTH_SOCK".to_string(), // SSH agent socket - commonly needed
        "SSH_AGENT_PID".to_string(), // SSH agent PID
        "SSH_CONNECTION".to_string(), // SSH connection info
        "SSH_CLIENT".to_string(),    // SSH client info
        "SSH_TTY".to_string(),       // SSH TTY
        // macOS specific
        "__CF_USER_TEXT_ENCODING".to_string(), // Core Foundation text encoding
        "__CFBundleIdentifier".to_string(),    // App bundle identifier
        "XPC_FLAGS".to_string(),
        "XPC_SERVICE_NAME".to_string(),
        // Homebrew
        "HOMEBREW_PREFIX".to_string(),
        "HOMEBREW_CELLAR".to_string(),
        "HOMEBREW_REPOSITORY".to_string(),
        // Terminal emulators
        "GHOSTTY_BIN_DIR".to_string(),
        "GHOSTTY_RESOURCES_DIR".to_string(),
        "GHOSTTY_SHELL_FEATURES".to_string(),
        "COLORTERM".to_string(),
        "COMMAND_MODE".to_string(),
        // Manual pages
        "MANPATH".to_string(),
        // Whi shell integration critical
        "WHI_SHELL_INITIALIZED".to_string(),
        "WHI_SESSION_PID".to_string(),
        "__WHI_BIN".to_string(),
        // Whi venv state (protected when in venv)
        "WHI_VENV_NAME".to_string(),
        "WHI_VENV_DIR".to_string(),
    ]
}

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

/// Get path to `protected_vars` file
pub fn get_protected_vars_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".whi").join("protected_vars"))
}

/// Get path to `protected_paths` file
pub fn get_protected_paths_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".whi").join("protected_paths"))
}

/// Get path to migration marker file
fn get_migration_marker_path() -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    Ok(PathBuf::from(home).join(".whi").join(".migrated"))
}

/// Check if migration has already been completed
fn is_migration_complete() -> Result<bool, String> {
    let marker_path = get_migration_marker_path()?;
    Ok(marker_path.exists())
}

/// Mark migration as complete by creating marker file
fn mark_migration_complete() -> Result<(), String> {
    let marker_path = get_migration_marker_path()?;

    // Create ~/.whi directory if needed
    if let Some(parent) = marker_path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Failed to create .whi directory: {e}"))?;
    }

    // Create simple marker file with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let content = format!("# Migration completed at Unix timestamp: {timestamp}\n");

    let mut atomic_file = AtomicFile::new(&marker_path)
        .map_err(|e| format!("Failed to create migration marker: {e}"))?;

    atomic_file
        .write_all(content.as_bytes())
        .map_err(|e| format!("Failed to write migration marker: {e}"))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit migration marker: {e}"))?;

    Ok(())
}

/// Parse `protected_vars` file
#[cfg(test)]
fn parse_protected_vars(content: &str) -> Result<Vec<String>, String> {
    parse_protected_items(content, "!protected.vars")
}

/// Parse `protected_paths` file
#[cfg(test)]
fn parse_protected_paths(content: &str) -> Result<Vec<PathBuf>, String> {
    parse_protected_items(content, "!protected.paths")
}

/// Format `protected_vars` for file
#[cfg(test)]
fn format_protected_vars(vars: &[String]) -> String {
    format_protected_items(vars, "!protected.vars")
}

/// Format `protected_paths` for file
#[cfg(test)]
fn format_protected_paths(paths: &[PathBuf]) -> String {
    format_protected_items(paths, "!protected.paths")
}

/// Critical environment variables that should never be removed from protection
fn critical_protected_vars() -> &'static [&'static str] {
    &["PATH", "HOME", "SHELL", "TERM", "USER"]
}

/// Validate that critical protected vars are present and warn if missing
fn validate_critical_vars(vars: &[String]) {
    let missing: Vec<&str> = critical_protected_vars()
        .iter()
        .filter(|&&critical| !vars.iter().any(|v| v == critical))
        .copied()
        .collect();

    if !missing.is_empty() {
        #[cfg(not(test))]
        {
            eprintln!(
                "Warning: Critical protected variables are missing from ~/.whi/protected_vars:"
            );
            eprintln!("  Missing: {}", missing.join(", "));
            eprintln!("  These are essential for shell stability. Consider adding them back.");
        }
    }
}

/// Load protected vars from file, or return defaults if file doesn't exist
pub fn load_protected_vars() -> Result<Vec<String>, String> {
    let path = get_protected_vars_path()?;
    load_protected_items(
        &path,
        "!protected.vars",
        default_protected_vars(),
        ensure_protected_vars_exists,
        Some(validate_critical_vars),
    )
}

/// Load protected paths from file, or return defaults if file doesn't exist
pub fn load_protected_paths() -> Result<Vec<PathBuf>, String> {
    let path = get_protected_paths_path()?;
    load_protected_items(
        &path,
        "!protected.paths",
        default_protected_paths(),
        ensure_protected_paths_exists,
        None,
    )
}

/// Create `protected_vars` file if it doesn't exist
pub fn ensure_protected_vars_exists() -> Result<(), String> {
    let path = get_protected_vars_path()?;
    ensure_protected_file_exists(&path, "!protected.vars", &default_protected_vars())
}

/// Create `protected_paths` file if it doesn't exist
pub fn ensure_protected_paths_exists() -> Result<(), String> {
    let path = get_protected_paths_path()?;
    ensure_protected_file_exists(&path, "!protected.paths", &default_protected_paths())
}

/// Save protected paths to file (used for migration)
pub fn save_protected_paths(paths: &[PathBuf]) -> Result<(), String> {
    let path = get_protected_paths_path()?;
    save_protected_items(paths, &path, "!protected.paths")
}

/// Migrate protected paths from config.toml to `protected_paths` file
/// Returns true if migration was performed
pub fn migrate_from_config_toml() -> Result<bool, String> {
    use std::io::Write;

    // Fast path: Check if migration is already complete
    if is_migration_complete()? {
        return Ok(false);
    }

    let home = env::var("HOME").map_err(|_| "HOME environment variable not set")?;
    let config_path = PathBuf::from(&home).join(".whi").join("config.toml");
    let protected_paths_file = get_protected_paths_path()?;

    // If protected_paths file already exists, migration already done (mark and return)
    if protected_paths_file.exists() {
        mark_migration_complete()?;
        return Ok(false);
    }

    // If config.toml doesn't exist, nothing to migrate (mark and return)
    if !config_path.exists() {
        mark_migration_complete()?;
        return Ok(false);
    }

    // Read config.toml
    let content =
        fs::read_to_string(&config_path).map_err(|e| format!("Failed to read config.toml: {e}"))?;

    // Check if it has [protected] section
    if !content.contains("[protected]") {
        mark_migration_complete()?;
        return Ok(false);
    }

    // Parse the protected paths from config
    let migrated_paths = parse_protected_paths_from_toml(&content);

    // Save to new protected_paths file
    save_protected_paths(&migrated_paths)?;

    // Rewrite config.toml without [protected] section
    let new_config_content = remove_protected_section(&content);
    let mut atomic_file = AtomicFile::new(&config_path)
        .map_err(|e| format!("Failed to open config.toml for migration: {e}"))?;

    atomic_file
        .write_all(new_config_content.as_bytes())
        .map_err(|e| format!("Failed to write migrated config.toml: {e}"))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit migrated config.toml: {e}"))?;

    // Add helpful comment to config.toml
    add_migration_comment_to_config(&config_path)?;

    // Mark migration as complete
    mark_migration_complete()?;

    Ok(true)
}

/// Parse protected paths from `TOML` content (for migration)
fn parse_protected_paths_from_toml(content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut in_protected_section = false;
    let mut in_array = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for [protected] section
        if trimmed == "[protected]" {
            in_protected_section = true;
            continue;
        }

        // Exit section if we hit another section header
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_protected_section = false;
            in_array = false;
            continue;
        }

        // If in protected section, look for paths array
        if in_protected_section {
            if trimmed.starts_with("paths") && trimmed.contains('[') {
                in_array = true;
                // Check for inline array: paths = ["/bin", "/usr/bin"]
                if trimmed.contains(']') {
                    // Parse inline array
                    if let Some(array_part) = trimmed.split('[').nth(1) {
                        if let Some(array_content) = array_part.split(']').next() {
                            for item in array_content.split(',') {
                                let cleaned = item.trim().trim_matches('"').trim();
                                if !cleaned.is_empty() {
                                    paths.push(PathBuf::from(cleaned));
                                }
                            }
                        }
                    }
                    in_array = false;
                }
                continue;
            }

            if in_array {
                if trimmed.contains(']') {
                    in_array = false;
                    continue;
                }
                // Parse array value
                let cleaned = trimmed.trim_end_matches(',').trim_matches('"').trim();
                if !cleaned.is_empty() {
                    paths.push(PathBuf::from(cleaned));
                }
            }
        }
    }

    paths
}

/// Add migration comment to config.toml explaining the migration
fn add_migration_comment_to_config(config_path: &PathBuf) -> Result<(), String> {
    use std::io::Write;

    // Read current content
    let current_content = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config.toml for comment: {e}"))?;

    // Check if migration comment already exists
    if current_content.contains("MIGRATION NOTE") {
        return Ok(()); // Comment already present
    }

    // Create migration comment
    let migration_comment = r"# MIGRATION NOTE: The [protected] section has been migrated to separate files.
# - Protected paths are now in: ~/.whi/protected_paths
# - Protected environment variables are now in: ~/.whi/protected_vars
# You can safely remove this comment and any remaining [protected] sections.
#
";

    // Prepend comment to content
    let new_content = format!("{migration_comment}{current_content}");

    // Write atomically
    let mut atomic_file = AtomicFile::new(config_path)
        .map_err(|e| format!("Failed to open config.toml for comment: {e}"))?;

    atomic_file
        .write_all(new_content.as_bytes())
        .map_err(|e| format!("Failed to write config.toml comment: {e}"))?;

    atomic_file
        .commit()
        .map_err(|e| format!("Failed to commit config.toml comment: {e}"))?;

    Ok(())
}

/// Remove [protected] section from `TOML` content
fn remove_protected_section(content: &str) -> String {
    let mut result = String::new();
    let mut in_protected_section = false;
    let mut in_protected_array = false;
    let mut last_was_empty = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Check for [protected] section start
        if trimmed == "[protected]" {
            in_protected_section = true;
            continue;
        }

        // Check for new section (exit protected section)
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_protected_section = false;
            in_protected_array = false;
        }

        // Skip lines in protected section
        if in_protected_section {
            // Check if we're entering paths array
            if trimmed.starts_with("paths") {
                in_protected_array = true;
                // Check for inline array
                if trimmed.contains(']') {
                    in_protected_array = false;
                }
                continue;
            }

            if in_protected_array {
                if trimmed.contains(']') {
                    in_protected_array = false;
                }
                continue;
            }

            // Skip comments and other content in protected section
            continue;
        }

        // Keep all other lines
        // Avoid double empty lines
        if trimmed.is_empty() {
            if last_was_empty {
                continue;
            }
            last_was_empty = true;
        } else {
            last_was_empty = false;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Shared mutex to serialize tests that manipulate HOME environment variable
    static HOME_TEST_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_parse_protected_vars() {
        let content = r#"!protected.vars
PATH
HOME
USER
SHELL
"#;
        let vars = parse_protected_vars(content).unwrap();
        assert_eq!(vars.len(), 4);
        assert_eq!(vars[0], "PATH");
        assert_eq!(vars[1], "HOME");
        assert_eq!(vars[2], "USER");
        assert_eq!(vars[3], "SHELL");
    }

    #[test]
    fn test_parse_protected_vars_with_comments() {
        let content = r#"!protected.vars
# System critical
PATH
HOME

# Shell
SHELL
"#;
        let vars = parse_protected_vars(content).unwrap();
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0], "PATH");
        assert_eq!(vars[1], "HOME");
        assert_eq!(vars[2], "SHELL");
    }

    #[test]
    fn test_parse_protected_paths() {
        let content = r#"!protected.paths
/usr/bin
/bin
/usr/local/bin
"#;
        let paths = parse_protected_paths(content).unwrap();
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], PathBuf::from("/usr/bin"));
        assert_eq!(paths[1], PathBuf::from("/bin"));
        assert_eq!(paths[2], PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn test_format_protected_vars() {
        let vars = vec!["PATH".to_string(), "HOME".to_string()];
        let content = format_protected_vars(&vars);
        assert!(content.starts_with("!protected.vars\n"));
        assert!(content.contains("PATH\n"));
        assert!(content.contains("HOME\n"));
    }

    #[test]
    fn test_format_protected_paths() {
        let paths = vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")];
        let content = format_protected_paths(&paths);
        assert!(content.starts_with("!protected.paths\n"));
        assert!(content.contains("/usr/bin\n"));
        assert!(content.contains("/bin\n"));
    }

    #[test]
    fn test_parse_missing_header() {
        let content = "PATH\nHOME\n";
        let result = parse_protected_vars(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("Missing !protected.vars header"));
    }

    #[test]
    fn test_default_protected_vars_includes_new_vars() {
        let defaults = default_protected_vars();
        assert!(defaults.contains(&"TERM".to_string()));
        assert!(defaults.contains(&"TERMINFO".to_string()));
        assert!(defaults.contains(&"TERM_PROGRAM".to_string()));
        assert!(defaults.contains(&"GHOSTTY_BIN_DIR".to_string()));
        assert!(defaults.contains(&"COLORTERM".to_string()));
        assert!(defaults.contains(&"HOMEBREW_PREFIX".to_string()));
    }

    #[test]
    fn test_parse_protected_paths_from_toml_multiline() {
        let content = r#"
[venv]
auto_activate_file = false

[protected]
paths = [
  "/usr/bin",
  "/bin",
  "/usr/local/bin",
]

[search]
executable_search_fuzzy = false
"#;
        let paths = parse_protected_paths_from_toml(content);
        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0], PathBuf::from("/usr/bin"));
        assert_eq!(paths[1], PathBuf::from("/bin"));
        assert_eq!(paths[2], PathBuf::from("/usr/local/bin"));
    }

    #[test]
    fn test_parse_protected_paths_from_toml_inline() {
        let content = r#"
[protected]
paths = ["/usr/bin", "/bin"]
"#;
        let paths = parse_protected_paths_from_toml(content);
        assert_eq!(paths.len(), 2);
        assert_eq!(paths[0], PathBuf::from("/usr/bin"));
        assert_eq!(paths[1], PathBuf::from("/bin"));
    }

    #[test]
    fn test_remove_protected_section_multiline() {
        let content = r#"# whi config
[venv]
auto_activate_file = false

[protected]
# Protected paths comment
paths = [
  "/usr/bin",
  "/bin",
]

[search]
executable_search_fuzzy = false
"#;
        let result = remove_protected_section(content);
        assert!(!result.contains("[protected]"));
        assert!(!result.contains("/usr/bin"));
        assert!(!result.contains("/bin"));
        assert!(result.contains("[venv]"));
        assert!(result.contains("[search]"));
        assert!(result.contains("auto_activate_file = false"));
        assert!(result.contains("executable_search_fuzzy = false"));
    }

    #[test]
    fn test_remove_protected_section_inline() {
        let content = r#"[venv]
auto_activate_file = true

[protected]
paths = ["/bin", "/usr/bin"]

[search]
executable_search_fuzzy = true
"#;
        let result = remove_protected_section(content);
        assert!(!result.contains("[protected]"));
        assert!(!result.contains("paths ="));
        assert!(result.contains("[venv]"));
        assert!(result.contains("[search]"));
    }

    #[test]
    fn test_migration_from_config_toml_full_workflow() {
        use tempfile::TempDir;

        // Lock to prevent parallel execution with other tests that manipulate HOME
        let _guard = HOME_TEST_LOCK.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();
        let whi_dir = temp_dir.path().join(".whi");
        fs::create_dir(&whi_dir).unwrap();

        // Create config.toml with [protected] section
        let config_path = whi_dir.join("config.toml");
        let config_content = r#"[venv]
auto_activate_file = false

[protected]
paths = [
  "/usr/bin",
  "/bin",
  "/usr/local/bin",
]

[search]
executable_search_fuzzy = false
"#;
        fs::write(&config_path, config_content).unwrap();

        // Override HOME to point to temp dir
        let old_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        // Run migration
        let migrated = migrate_from_config_toml().unwrap();
        assert!(migrated, "Migration should return true when performed");

        // Verify protected_paths file was created
        let protected_paths_file = whi_dir.join("protected_paths");
        assert!(
            protected_paths_file.exists(),
            "protected_paths file should exist after migration"
        );

        // Verify content of protected_paths file
        let paths_content = fs::read_to_string(&protected_paths_file).unwrap();
        assert!(paths_content.contains("!protected.paths"));
        assert!(paths_content.contains("/usr/bin"));
        assert!(paths_content.contains("/bin"));
        assert!(paths_content.contains("/usr/local/bin"));

        // Verify [protected] section was removed from config.toml
        let new_config_content = fs::read_to_string(&config_path).unwrap();

        // Verify migration comment was added
        assert!(
            new_config_content.contains("MIGRATION NOTE"),
            "Migration comment should be present"
        );
        assert!(
            new_config_content.contains("~/.whi/protected_paths"),
            "Migration comment should reference protected_paths"
        );

        // Verify the actual [protected] section was removed (not just the word in comments)
        let lines_with_protected_section: Vec<&str> = new_config_content
            .lines()
            .filter(|line| !line.trim().starts_with('#'))
            .filter(|line| line.contains("[protected]"))
            .collect();
        assert!(
            lines_with_protected_section.is_empty(),
            "Protected section header should be removed from config.toml"
        );
        assert!(
            !new_config_content
                .lines()
                .filter(|line| !line.trim().starts_with('#'))
                .any(|line| line.contains("paths =")),
            "Paths array should be removed from config.toml"
        );

        // Verify other sections are preserved
        assert!(new_config_content.contains("[venv]"));
        assert!(new_config_content.contains("[search]"));
        assert!(new_config_content.contains("auto_activate_file = false"));

        // Verify migration marker was created
        let marker_path = whi_dir.join(".migrated");
        assert!(marker_path.exists(), "Migration marker should exist");

        // Verify loading the migrated paths works
        let loaded_paths = load_protected_paths().unwrap();
        assert_eq!(loaded_paths.len(), 3);
        assert_eq!(loaded_paths[0], PathBuf::from("/usr/bin"));
        assert_eq!(loaded_paths[1], PathBuf::from("/bin"));
        assert_eq!(loaded_paths[2], PathBuf::from("/usr/local/bin"));

        // Running migration again should return false (already done)
        let migrated_again = migrate_from_config_toml().unwrap();
        assert!(
            !migrated_again,
            "Migration should return false when already done"
        );

        // Restore HOME
        if let Some(home) = old_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_migration_when_no_protected_section() {
        use tempfile::TempDir;

        // Lock to prevent parallel execution with other tests that manipulate HOME
        let _guard = HOME_TEST_LOCK.lock().unwrap();

        let temp_dir = TempDir::new().unwrap();
        let whi_dir = temp_dir.path().join(".whi");
        fs::create_dir(&whi_dir).unwrap();

        // Create config.toml WITHOUT [protected] section
        let config_path = whi_dir.join("config.toml");
        let config_content = r#"[venv]
auto_activate_file = false

[search]
executable_search_fuzzy = false
"#;
        fs::write(&config_path, config_content).unwrap();

        let old_home = env::var("HOME").ok();
        env::set_var("HOME", temp_dir.path());

        // Run migration - should return false (nothing to migrate)
        let migrated = migrate_from_config_toml().unwrap();
        assert!(
            !migrated,
            "Migration should return false when no [protected] section"
        );

        // Verify protected_paths file was NOT created
        let protected_paths_file = whi_dir.join("protected_paths");
        assert!(
            !protected_paths_file.exists(),
            "protected_paths file should not be created when nothing to migrate"
        );

        // Verify migration marker was created (to avoid repeated checks)
        let marker_path = whi_dir.join(".migrated");
        assert!(
            marker_path.exists(),
            "Migration marker should exist even when nothing to migrate"
        );

        // Restore HOME
        if let Some(home) = old_home {
            env::set_var("HOME", home);
        } else {
            env::remove_var("HOME");
        }
    }

    #[test]
    fn test_critical_vars_validation() {
        let complete_vars = vec![
            "PATH".to_string(),
            "HOME".to_string(),
            "SHELL".to_string(),
            "TERM".to_string(),
            "USER".to_string(),
            "EXTRA".to_string(),
        ];

        // Should not panic or warn (in test mode warnings are suppressed)
        validate_critical_vars(&complete_vars);

        let incomplete_vars = vec!["PATH".to_string(), "EXTRA".to_string()];

        // Should not panic (warnings are suppressed in test mode)
        validate_critical_vars(&incomplete_vars);
    }

    #[test]
    fn test_robust_comment_handling_like_whifiles() {
        // Test that protected files can handle extensive comments just like whifiles
        let content = r"# Protected paths configuration
#
# This file specifies which paths should never be removed from PATH
# to prevent system breakage
#
!protected.paths

# System critical paths
/usr/bin
/bin

   # Database tools (indented comment)
/usr/local/bin

# More paths below
/usr/sbin


# Empty lines above are fine
/sbin

# End of file
";

        let paths = parse_protected_paths(content).unwrap();
        assert_eq!(paths.len(), 5);
        assert_eq!(paths[0], PathBuf::from("/usr/bin"));
        assert_eq!(paths[1], PathBuf::from("/bin"));
        assert_eq!(paths[2], PathBuf::from("/usr/local/bin"));
        assert_eq!(paths[3], PathBuf::from("/usr/sbin"));
        assert_eq!(paths[4], PathBuf::from("/sbin"));
    }

    #[test]
    fn test_protected_vars_robust_comments() {
        // Verify protected_vars handles same commenting style as whifiles
        let content = r"# Protected environment variables
#
# Variables listed here will never be unset by !env.replace
#
!protected.vars

# System critical variables
PATH
HOME

# Shell information
SHELL
USER

   # Terminal settings (indented)
TERM
LANG


# SSH agent (empty lines above)
SSH_AUTH_SOCK

# whi integration
WHI_SHELL_INITIALIZED
";

        let vars = parse_protected_vars(content).unwrap();
        assert_eq!(vars.len(), 8);
        assert!(vars.contains(&"PATH".to_string()));
        assert!(vars.contains(&"HOME".to_string()));
        assert!(vars.contains(&"SHELL".to_string()));
        assert!(vars.contains(&"USER".to_string()));
        assert!(vars.contains(&"TERM".to_string()));
        assert!(vars.contains(&"LANG".to_string()));
        assert!(vars.contains(&"SSH_AUTH_SOCK".to_string()));
        assert!(vars.contains(&"WHI_SHELL_INITIALIZED".to_string()));
    }

    #[test]
    fn test_inline_comments_in_protected_paths() {
        // Test that inline comments work like in whifiles
        let content = r"!protected.paths
/usr/local/bin
/usr/bin
/bin
/usr/sbin     # inline comment
/sbin # inline comment with single space
/usr/local/sbin#no space before comment
";

        let paths = parse_protected_paths(content).unwrap();
        assert_eq!(paths.len(), 6);
        assert_eq!(paths[0], PathBuf::from("/usr/local/bin"));
        assert_eq!(paths[1], PathBuf::from("/usr/bin"));
        assert_eq!(paths[2], PathBuf::from("/bin"));
        assert_eq!(paths[3], PathBuf::from("/usr/sbin"));
        assert_eq!(paths[4], PathBuf::from("/sbin"));
        assert_eq!(paths[5], PathBuf::from("/usr/local/sbin"));
    }

    #[test]
    fn test_inline_comments_in_protected_vars() {
        let content = r"!protected.vars
PATH     # Absolutely critical
HOME # User home directory
SHELL # Current shell
";

        let vars = parse_protected_vars(content).unwrap();
        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0], "PATH");
        assert_eq!(vars[1], "HOME");
        assert_eq!(vars[2], "SHELL");
    }
}
