use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::DirBuilderExt;

use crate::atomic_file::AtomicFile;
use crate::history::HistoryContext;
use crate::path_guard::PathGuard;

const WHI_FILE: &str = "whifile";

/// Represents a single environment variable change operation
#[derive(Debug, Clone)]
pub enum EnvChange {
    /// Set a variable to a value
    Set(String, String),
    /// Unset a variable
    Unset(String),
    /// Source a script file
    Source(String),
    /// Execute a command (typically during exit)
    Run(String),
}

#[derive(Debug, Clone)]
pub struct VenvTransition {
    pub new_path: String,
    pub env_changes: Vec<EnvChange>,
    pub needs_pyenv_deactivate: bool,
}

/// Returns the list of protected environment variables that should never be unset
///
/// This guard prevents users from accidentally unsetting critical env vars via whifiles.
/// Similar to how whi's `PATH` is always protected (can't be deleted from PATH).
///
/// IMPORTANT: When implementing !env.saved functionality (saving/restoring env vars),
/// ensure it also uses this guard to avoid saving/restoring protected variables.
///
/// Loads from `~/.whi/protected_vars` file, falls back to hardcoded defaults if load fails.
fn protected_env_vars() -> Vec<String> {
    use crate::protected_config::load_protected_vars;

    // Load from file, fall back to hardcoded defaults if it fails
    load_protected_vars().unwrap_or_else(|e| {
        // Warn about fallback in non-test environments
        #[cfg(not(test))]
        {
            eprintln!("Warning: Failed to load protected environment variables: {e}");
            eprintln!("Using hardcoded defaults. Custom protected vars may not be active.");
        }

        #[cfg(test)]
        let _ = e; // Suppress unused warning in tests

        // Fallback to hardcoded defaults
        vec![
            // System critical - universal
            "PATH".to_string(),
            "HOME".to_string(),
            "USER".to_string(),
            "LOGNAME".to_string(),
            "SHELL".to_string(),
            "TERM".to_string(),
            "TERMINFO".to_string(),
            "TERM_PROGRAM".to_string(),
            "TERM_PROGRAM_VERSION".to_string(),
            "LANG".to_string(),
            "LC_ALL".to_string(),
            "LC_CTYPE".to_string(),
            "LC_MESSAGES".to_string(),
            "LC_NUMERIC".to_string(),
            "LC_COLLATE".to_string(),
            "LC_TIME".to_string(),
            "IFS".to_string(),
            // Shell state
            "PWD".to_string(),
            "OLDPWD".to_string(),
            "SHLVL".to_string(),
            // Temp directories
            "TMPDIR".to_string(),
            "TMP".to_string(),
            "TEMP".to_string(),
            // Display/GUI
            "DISPLAY".to_string(),
            "WAYLAND_DISPLAY".to_string(),
            "XDG_RUNTIME_DIR".to_string(),
            "XDG_SESSION_TYPE".to_string(),
            "XDG_DATA_DIRS".to_string(),
            "XAUTHORITY".to_string(),
            "DBUS_SESSION_BUS_ADDRESS".to_string(),
            // SSH
            "SSH_AUTH_SOCK".to_string(),
            "SSH_AGENT_PID".to_string(),
            "SSH_CONNECTION".to_string(),
            "SSH_CLIENT".to_string(),
            "SSH_TTY".to_string(),
            // macOS specific
            "__CF_USER_TEXT_ENCODING".to_string(),
            "__CFBundleIdentifier".to_string(),
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
            // Whi venv state
            "WHI_VENV_DIR".to_string(),
            "WHI_PYENV_MANAGED".to_string(),
            "VIRTUAL_ENV_PROMPT".to_string(),
            "VIRTUAL_ENV".to_string(),
        ]
    })
}

/// Check if we're in a venv
#[must_use]
pub fn is_in_venv() -> bool {
    env::var("VIRTUAL_ENV_PROMPT").is_ok()
}

/// Returns the directory backing the active whi-managed venv, if any.
///
/// Uses whi-owned metadata so Python's activate script cannot clobber
/// the identifier we rely on for history bookkeeping.
#[must_use]
pub fn current_venv_dir() -> Option<PathBuf> {
    if !is_in_venv() {
        return None;
    }

    if let Ok(dir) = env::var("WHI_VENV_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    let session_pid = get_session_pid();
    load_saved_venv_dir(session_pid).ok().flatten()
}

/// Get session `PID` from environment
fn get_session_pid() -> u32 {
    env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id)
}

/// Get session directory for this session
fn get_session_dir(session_pid: u32) -> io::Result<PathBuf> {
    use crate::system;
    use std::env;

    // Try XDG_RUNTIME_DIR first (standard for user-specific runtime files)
    let base_dir = env::var("XDG_RUNTIME_DIR")
        .or_else(|_| env::var("TMPDIR"))
        .unwrap_or_else(|_| "/tmp".to_string());

    // Use UID for additional security
    let uid = system::get_user_id()
        .map_err(|e| io::Error::other(format!("Failed to get user ID: {e}")))?;

    let session_dir = PathBuf::from(format!("{base_dir}/whi-{uid}/session_{session_pid}"));

    // Create directory if it doesn't exist
    if !session_dir.exists() {
        #[cfg(unix)]
        {
            std::fs::DirBuilder::new()
                .mode(0o700)
                .recursive(true)
                .create(&session_dir)?;
        }

        #[cfg(not(unix))]
        {
            fs::create_dir_all(&session_dir)?;
        }
    }

    Ok(session_dir)
}

/// Get venv restore file path
fn get_venv_restore_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("venv_restore"))
}

/// Get venv dir file path
fn get_venv_dir_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("venv_dir"))
}

/// Get venv env keys file path
fn get_venv_env_keys_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("venv_env_keys"))
}

/// Get exit commands file path
fn get_venv_exit_commands_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("venv_exit_commands"))
}

/// Save `PATH` for venv restore
fn save_venv_restore(session_pid: u32, path: &str) -> io::Result<()> {
    let restore_file = get_venv_restore_file(session_pid)?;
    fs::write(restore_file, path)?;
    Ok(())
}

/// Restore venv `PATH`
fn restore_venv_path(session_pid: u32) -> io::Result<String> {
    let restore_file = get_venv_restore_file(session_pid)?;
    let path = fs::read_to_string(restore_file)?;
    Ok(path.trim().to_string())
}

/// Save venv info (directory)
fn save_venv_info(session_pid: u32, dir: &Path) -> io::Result<()> {
    let venv_dir_file = get_venv_dir_file(session_pid)?;
    fs::write(venv_dir_file, dir.to_string_lossy().as_bytes())?;

    Ok(())
}

/// Load saved venv directory (if any)
fn load_saved_venv_dir(session_pid: u32) -> io::Result<Option<PathBuf>> {
    let dir_file = get_venv_dir_file(session_pid)?;
    if !dir_file.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(dir_file)?;
    let trimmed = content.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(PathBuf::from(trimmed)))
    }
}

/// Save env var keys for venv (so we know what to unset on exit)
fn save_venv_env_keys(session_pid: u32, keys: &[String]) -> io::Result<()> {
    let env_keys_file = get_venv_env_keys_file(session_pid)?;
    fs::write(env_keys_file, keys.join("\n"))?;
    Ok(())
}

/// Load env var keys for venv
fn load_venv_env_keys(session_pid: u32) -> io::Result<Vec<String>> {
    let env_keys_file = get_venv_env_keys_file(session_pid)?;
    if !env_keys_file.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(env_keys_file)?;
    Ok(content
        .lines()
        .map(std::string::ToString::to_string)
        .collect())
}

/// Save exit commands to replay on `whi exit`
fn save_venv_exit_commands(session_pid: u32, commands: &[String]) -> io::Result<()> {
    let file = get_venv_exit_commands_file(session_pid)?;
    if commands.is_empty() {
        if file.exists() {
            let _ = fs::remove_file(&file);
        }
        return Ok(());
    }

    fs::write(file, commands.join("\n"))?;
    Ok(())
}

/// Load exit commands (best-effort)
fn load_venv_exit_commands(session_pid: u32) -> Vec<String> {
    get_venv_exit_commands_file(session_pid)
        .ok()
        .and_then(|path| fs::read_to_string(path).ok())
        .map(|content| {
            content
                .lines()
                .filter(|line| !line.trim().is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Get pyenv deactivate flag file path
fn get_pyenv_deactivate_file(session_pid: u32) -> io::Result<PathBuf> {
    Ok(get_session_dir(session_pid)?.join("pyenv_active"))
}

/// Save pyenv deactivate flag (indicates pyenv was activated)
fn save_pyenv_deactivate_flag(session_pid: u32) -> io::Result<()> {
    let flag_file = get_pyenv_deactivate_file(session_pid)?;
    fs::write(flag_file, "1")?;
    Ok(())
}

/// Load pyenv deactivate flag (returns true if pyenv was activated)
fn load_pyenv_deactivate_flag(session_pid: u32) -> bool {
    get_pyenv_deactivate_file(session_pid)
        .ok()
        .and_then(|f| fs::read_to_string(f).ok())
        .is_some()
}

/// Clear pyenv deactivate flag
fn clear_pyenv_deactivate_flag(session_pid: u32) {
    if let Ok(file) = get_pyenv_deactivate_file(session_pid) {
        let _ = fs::remove_file(file);
    }
}

/// Clear venv info
fn clear_venv_info(session_pid: u32) {
    if let Ok(restore_file) = get_venv_restore_file(session_pid) {
        let _ = fs::remove_file(restore_file);
    }
    if let Ok(dir_file) = get_venv_dir_file(session_pid) {
        let _ = fs::remove_file(dir_file);
    }
    if let Ok(env_keys_file) = get_venv_env_keys_file(session_pid) {
        let _ = fs::remove_file(env_keys_file);
    }
    if let Ok(exit_commands_file) = get_venv_exit_commands_file(session_pid) {
        let _ = fs::remove_file(exit_commands_file);
    }
    clear_pyenv_deactivate_flag(session_pid);
}

/// Expand environment variables and command substitutions in a value
/// Supports: $VAR, ${VAR}, $(command), `command`, and ~ expansion
#[must_use]
pub fn expand_shell_vars(value: &str) -> String {
    let mut result = String::new();
    let mut chars = value.chars().peekable();
    let mut at_start = true;

    while let Some(ch) = chars.next() {
        if ch == '~' && (at_start || result.ends_with(':') || result.ends_with(' ')) {
            // Tilde expansion: ~ or ~/ at start or after : or space
            if chars.peek() == Some(&'/') || chars.peek().is_none() || chars.peek() == Some(&':') {
                // Simple ~ or ~/ or ~: -> expand to $HOME
                if let Ok(home) = env::var("HOME") {
                    result.push_str(&home);
                } else {
                    result.push('~');
                }
            } else {
                // ~username not supported, just keep literal
                result.push('~');
            }
            at_start = false;
        } else if ch == '$' {
            if chars.peek() == Some(&'(') {
                // Command substitution: $(...)
                chars.next(); // consume '('
                let mut cmd = String::new();
                let mut depth = 1;

                for c in chars.by_ref() {
                    if c == '(' {
                        depth += 1;
                        cmd.push(c);
                    } else if c == ')' {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                        cmd.push(c);
                    } else {
                        cmd.push(c);
                    }
                }

                // Execute command and capture output
                if let Ok(output) = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .output()
                {
                    if output.status.success() {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        result.push_str(stdout.trim());
                    }
                }
            } else if chars.peek() == Some(&'{') {
                // ${VAR} syntax
                chars.next(); // consume '{'
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
                // $VAR syntax
                let mut var_name = String::new();

                while let Some(&c) = chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
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
        } else if ch == '`' {
            // Backtick command substitution: `...`
            let mut cmd = String::new();

            for c in chars.by_ref() {
                if c == '`' {
                    break;
                }
                cmd.push(c);
            }

            if let Ok(output) = std::process::Command::new("sh")
                .arg("-c")
                .arg(&cmd)
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    result.push_str(stdout.trim());
                }
            }
            at_start = false;
        } else {
            result.push(ch);
            at_start = false;
        }
    }

    result
}

/// Create whifile from current `PATH`
pub fn create_file(force: bool) -> io::Result<()> {
    use crate::path_file::default_whifile_template;
    use crate::protected_config::load_protected_paths;

    let whi_file = Path::new(WHI_FILE);

    // Check for existing whifile
    if whi_file.exists() && !force {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "whifile already exists. Use -f/--force to replace it",
        ));
    }

    // Load protected paths from file (creates default file if doesn't exist)
    let protected_paths: Vec<String> = load_protected_paths()
        .unwrap_or_default()
        .iter()
        .map(|p| p.to_string_lossy().to_string())
        .collect();

    let protected_count = protected_paths.len();

    // Use template with protected paths included
    let template = default_whifile_template(&protected_paths);

    // Write atomically
    let mut atomic_file = AtomicFile::new(whi_file)?;
    atomic_file.write_all(template.as_bytes())?;
    atomic_file.commit()?;

    println!("Created whifile template with {protected_count} protected paths - edit to customize");

    Ok(())
}

/// Source venv from specific path (used by shell integration)
/// Auto-upgrade whifile from old format to new format
fn auto_upgrade_whifile(
    path_file: &Path,
    parsed: &crate::path_file::ParsedPathFile,
) -> io::Result<()> {
    use crate::atomic_file::AtomicFile;
    use crate::path_file::format_path_file_with_env;

    let path_str = if let Some(ref replace) = parsed.path.replace {
        replace.join(":")
    } else {
        String::new()
    };

    let env_vars: Vec<(String, String)> = parsed
        .env
        .operations
        .iter()
        .filter_map(|op| match op {
            crate::path_file::EnvOperation::Set(k, v) => Some((k.clone(), v.clone())),
            _ => None,
        })
        .collect();

    let new_content = format_path_file_with_env(&path_str, &env_vars);
    let mut atomic_file = AtomicFile::new(path_file)?;
    atomic_file.write_all(new_content.as_bytes())?;
    atomic_file.commit()?;
    Ok(())
}

fn process_env_operations(operations: &[crate::path_file::EnvOperation]) -> Vec<EnvChange> {
    use crate::path_file::EnvOperation;
    use std::collections::HashMap;

    let mut changes = Vec::new();
    // Track simulated environment state (starts with current process env)
    let mut simulated_env: HashMap<String, String> = env::vars().collect();
    let protected = protected_env_vars();

    for operation in operations {
        match operation {
            EnvOperation::Replace(replace_vars) => {
                // Unset all non-protected vars that aren't in the replace list
                for key in simulated_env.keys() {
                    if !protected.contains(key) && !replace_vars.iter().any(|(k, _)| k == key) {
                        changes.push(EnvChange::Unset(key.clone()));
                    }
                }

                // Clear simulated env of non-protected vars
                simulated_env.retain(|k, _| protected.contains(k));

                // Set all replace vars
                for (key, value) in replace_vars {
                    let expanded_value = expand_shell_vars(value);
                    changes.push(EnvChange::Set(key.clone(), expanded_value.clone()));
                    simulated_env.insert(key.clone(), expanded_value);
                }
            }
            EnvOperation::Set(key, value) => {
                let expanded_value = expand_shell_vars(value);
                changes.push(EnvChange::Set(key.clone(), expanded_value.clone()));
                simulated_env.insert(key.clone(), expanded_value);
            }
            EnvOperation::Unset(key) => {
                changes.push(EnvChange::Unset(key.clone()));
                simulated_env.remove(key);
            }
        }
    }

    changes
}

/// Process extra directives and return (`env_changes`, `needs_pyenv_deactivate`)
fn process_extra_directives(
    directives: &[crate::path_file::ExtraDirective],
) -> (Vec<EnvChange>, bool, Vec<String>) {
    use crate::path_file::ExtraDirective;
    use crate::shell_detect::{detect_current_shell, Shell};

    let mut env_changes = Vec::new();
    let mut needs_pyenv_deactivate = false;
    let mut exit_commands = Vec::new();

    for directive in directives {
        match directive {
            ExtraDirective::Source { script, on_exit } => {
                let expanded_path = expand_shell_vars(script);
                if Path::new(&expanded_path).exists() {
                    env_changes.push(EnvChange::Source(expanded_path));
                    if let Some(cmd) = on_exit {
                        exit_commands.push(cmd.clone());
                    }
                } else {
                    eprintln!("Warning: $source script not found: {expanded_path}");
                    eprintln!("         Skipping script; whi environment still activated.");
                }
            }
            ExtraDirective::PyEnv(venv_dir) => {
                let expanded_dir = expand_shell_vars(venv_dir);

                // Detect shell to choose correct activate script
                let shell = detect_current_shell().unwrap_or(Shell::Bash);
                let activate_name = match shell {
                    Shell::Fish => "activate.fish",
                    _ => "activate",
                };

                // Smart path resolution: handle both .venv and .venv/bin
                let activate_script = if expanded_dir.ends_with("/bin") {
                    // User provided .venv/bin, use directly
                    format!("{expanded_dir}/{activate_name}")
                } else {
                    // User provided .venv, append /bin
                    format!("{expanded_dir}/bin/{activate_name}")
                };

                // Check if activate script exists
                let mut added = false;
                if Path::new(&activate_script).exists() {
                    env_changes.push(EnvChange::Source(activate_script));
                    added = true;
                } else {
                    // Try alternative if first attempt failed
                    let alternative = if expanded_dir.ends_with("/bin") {
                        // They gave us /bin, try without it
                        let base = expanded_dir.trim_end_matches("/bin");
                        format!("{base}/bin/{activate_name}")
                    } else {
                        // They gave us base, try with /bin added
                        format!("{expanded_dir}/{activate_name}")
                    };

                    if Path::new(&alternative).exists() {
                        env_changes.push(EnvChange::Source(alternative));
                        added = true;
                    } else {
                        eprintln!(
                            "Warning: Python venv activate script not found for $pyenv {venv_dir}"
                        );
                        eprintln!("         Tried: {activate_script}");
                        eprintln!("         Also tried: {alternative}");
                        eprintln!("         Expected structure: <venv>/bin/{activate_name}");
                        eprintln!(
                            "         Skipping $pyenv activation; whi environment still initialized."
                        );
                    }
                }

                if added {
                    needs_pyenv_deactivate = true;
                }
            }
        }
    }

    (env_changes, needs_pyenv_deactivate, exit_commands)
}

pub fn source_from_path(dir_path: &str) -> io::Result<VenvTransition> {
    use crate::path_file::{apply_path_sections, parse_path_file};

    if is_in_venv() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "Already in a venv. Run 'whi exit' first",
        ));
    }

    let dir = Path::new(dir_path);
    let whi_file = dir.join(WHI_FILE);

    let path_file = if whi_file.exists() {
        whi_file
    } else {
        return Err(io::Error::new(io::ErrorKind::NotFound, "No whifile found"));
    };

    let file_content = fs::read_to_string(&path_file)?;

    let needs_upgrade = file_content
        .lines()
        .find(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .is_some_and(|first_line| first_line.trim() == "PATH!" || first_line.trim() == "ENV!");

    let parsed = parse_path_file(&file_content).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Failed to parse {}: {}", path_file.display(), e),
        )
    })?;

    if needs_upgrade {
        auto_upgrade_whifile(&path_file, &parsed)?;
    }

    // Get current session PATH BEFORE activation (used as base for prepend/append and for restore)
    let session_pid = get_session_pid();
    let current_path = env::var("PATH").unwrap_or_default();

    // Apply PATH sections to session PATH
    let computed_path = apply_path_sections(&current_path, &parsed.path)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    // Expand shell variables in computed PATH entries
    let expanded_path = computed_path
        .split(':')
        .map(expand_shell_vars)
        .collect::<Vec<_>>()
        .join(":");

    // Apply path guard to preserve critical binaries (whi, zoxide)
    let guarded_path = PathGuard::default().ensure_protected_paths(&current_path, expanded_path);

    // Get directory name for venv name
    let venv_name = dir.file_name().map_or_else(
        || "whi-venv".to_string(),
        |s| s.to_string_lossy().into_owned(),
    );

    // Save current session PATH for restore (BEFORE activation)
    save_venv_restore(session_pid, &current_path)?;
    save_venv_info(session_pid, dir)?;

    // Handle environment variables - preserve operation order
    let mut env_changes = vec![
        EnvChange::Set("VIRTUAL_ENV_PROMPT".to_string(), venv_name),
        EnvChange::Set("VIRTUAL_ENV".to_string(), dir.display().to_string()),
        EnvChange::Set("WHI_VENV_DIR".to_string(), dir.display().to_string()),
    ];

    // Process user-defined env operations (preserves order and tracks state)
    let user_env_changes = process_env_operations(&parsed.env.operations);

    // Extract keys of SET operations for saving (so we know what to unset on exit)
    let env_keys: Vec<String> = user_env_changes
        .iter()
        .filter_map(|change| match change {
            EnvChange::Set(key, _) => Some(key.clone()),
            EnvChange::Unset(_) | EnvChange::Source(_) | EnvChange::Run(_) => None,
        })
        .collect();

    if !env_keys.is_empty() {
        save_venv_env_keys(session_pid, &env_keys)?;
    }

    // Append user env changes to maintain order
    env_changes.extend(user_env_changes);

    // Process extra directives (sourcing scripts, python venvs)
    let (extra_changes, needs_pyenv_deactivate, exit_commands) =
        process_extra_directives(&parsed.extra.directives);
    env_changes.extend(extra_changes);

    save_venv_exit_commands(session_pid, &exit_commands)?;

    // Save pyenv deactivation flag if needed
    if needs_pyenv_deactivate {
        save_pyenv_deactivate_flag(session_pid)?;
        env_changes.push(EnvChange::Set(
            "WHI_PYENV_MANAGED".to_string(),
            "1".to_string(),
        ));
    }

    // Reset venv history with computed PATH (isolated from global history)
    HistoryContext::venv(session_pid, dir)
        .and_then(|ctx| ctx.reset_with_initial(&guarded_path))
        .map_err(io::Error::other)?;

    Ok(VenvTransition {
        new_path: guarded_path,
        env_changes,
        needs_pyenv_deactivate,
    })
}

/// Source venv from pwd (whifile) - convenience wrapper
pub fn source() -> io::Result<VenvTransition> {
    let pwd = env::current_dir()?;
    source_from_path(&pwd.to_string_lossy())
}

/// Exit venv
pub fn exit_venv() -> io::Result<VenvTransition> {
    if !is_in_venv() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "Not in a venv"));
    }

    let session_pid = get_session_pid();
    let restored_path = restore_venv_path(session_pid)?;

    // Load env var keys that were set by the venv
    let env_keys = load_venv_env_keys(session_pid).unwrap_or_default();

    // Load scripted exit commands (best effort)
    let exit_commands = load_venv_exit_commands(session_pid);

    // Check if pyenv needs deactivation
    let needs_pyenv_deactivate = load_pyenv_deactivate_flag(session_pid);

    // Clear venv info
    clear_venv_info(session_pid);

    // Build env_changes: run scripted exits first, then unset whi vars + user vars
    let mut env_changes: Vec<EnvChange> = exit_commands.into_iter().map(EnvChange::Run).collect();

    env_changes.push(EnvChange::Unset("VIRTUAL_ENV_PROMPT".to_string()));
    env_changes.push(EnvChange::Unset("VIRTUAL_ENV".to_string()));
    env_changes.push(EnvChange::Unset("WHI_VENV_DIR".to_string()));

    // Add user env vars to unset
    for key in env_keys {
        env_changes.push(EnvChange::Unset(key));
    }

    if needs_pyenv_deactivate {
        env_changes.push(EnvChange::Unset("WHI_PYENV_MANAGED".to_string()));
    }

    Ok(VenvTransition {
        new_path: restored_path,
        env_changes,
        needs_pyenv_deactivate,
    })
}

/// Update the stored restore `PATH` for the active venv
pub fn update_restore_path(new_path: &str) -> io::Result<()> {
    if !is_in_venv() {
        return Ok(());
    }

    let session_pid = get_session_pid();
    save_venv_restore(session_pid, new_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::path::Path;
    use std::sync::MutexGuard;
    use tempfile::TempDir;

    fn env_guard() -> MutexGuard<'static, ()> {
        crate::test_utils::env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn test_create_file() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();
        env::set_current_dir(&temp_path).unwrap();
        env::set_var("PATH", "/usr/bin:/bin");
        env::set_var("WHI_SESSION_PID", "12345");

        // First create should succeed
        assert!(create_file(false).is_ok());
        assert!(temp_path.join(WHI_FILE).exists());

        // Second create without --force should fail
        let result = create_file(false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));

        // With --force should succeed
        assert!(create_file(true).is_ok());

        // Cleanup
        env::remove_var("WHI_SESSION_PID");
    }

    #[test]
    fn test_update_restore_path_refreshes_backup() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_var("VIRTUAL_ENV_PROMPT", "test-venv");
        env::set_var("WHI_SESSION_PID", "4242");

        save_venv_restore(4242, "/old:path").unwrap();
        update_restore_path("/new:path").unwrap();
        let restored = restore_venv_path(4242).unwrap();
        assert_eq!(restored, "/new:path");

        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("WHI_SESSION_PID");
        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_current_venv_dir_prefers_env_then_file() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();
        let session_pid = 1312u32;

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_var("WHI_SESSION_PID", session_pid.to_string());
        env::set_var("VIRTUAL_ENV_PROMPT", "test-venv");

        env::set_var("WHI_VENV_DIR", "/tmp/from-env");
        let from_env = current_venv_dir();
        assert_eq!(from_env.as_deref(), Some(Path::new("/tmp/from-env")));

        env::remove_var("WHI_VENV_DIR");
        assert!(current_venv_dir().is_none());

        save_venv_info(session_pid, Path::new("/tmp/from-file")).unwrap();
        let from_file = current_venv_dir();
        assert_eq!(from_file.as_deref(), Some(Path::new("/tmp/from-file")));

        clear_venv_info(session_pid);

        env::remove_var("WHI_SESSION_PID");
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("WHI_VENV_DIR");
        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_source_from_path_reads_whi_file() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_current_dir(temp_dir.path()).unwrap();
        env::set_var("WHI_SESSION_PID", "7777");
        env::set_var("PATH", "/usr/bin:/bin");
        env::remove_var("VIRTUAL_ENV_PROMPT");

        fs::write(WHI_FILE, "PATH!\n/usr/bin\n/bin\n\nENV!\n").unwrap();

        let transition = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();
        assert_eq!(transition.new_path, "/usr/bin:/bin");

        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");
        env::remove_var("WHI_SESSION_PID");
        env::remove_var("PATH");

        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_expand_shell_vars() {
        let _guard = env_guard();
        let old_test_var = env::var("TEST_VAR").ok();
        let old_home = env::var("HOME").ok();
        let old_user = env::var("USER").ok();
        env::set_var("TEST_VAR", "hello");
        env::set_var("HOME", "/home/user");
        env::set_var("USER", "testuser");

        assert_eq!(expand_shell_vars("$TEST_VAR"), "hello");
        assert_eq!(expand_shell_vars("${TEST_VAR}"), "hello");
        assert_eq!(
            expand_shell_vars("prefix $TEST_VAR suffix"),
            "prefix hello suffix"
        );
        assert_eq!(expand_shell_vars("$HOME/dir"), "/home/user/dir");
        assert_eq!(
            expand_shell_vars("/Users/$USER/.bun/bin"),
            "/Users/testuser/.bun/bin"
        );
        assert_eq!(expand_shell_vars("$(echo test)"), "test");
        assert_eq!(expand_shell_vars("`echo test`"), "test");

        // Tilde expansion
        assert_eq!(expand_shell_vars("~"), "/home/user");
        assert_eq!(expand_shell_vars("~/config"), "/home/user/config");
        assert_eq!(expand_shell_vars("~/.bashrc"), "/home/user/.bashrc");
        assert_eq!(
            expand_shell_vars("/usr/bin:~/bin"),
            "/usr/bin:/home/user/bin"
        );
        assert_eq!(
            expand_shell_vars("prefix ~/suffix"),
            "prefix /home/user/suffix"
        );
        assert_eq!(expand_shell_vars("~:~/bin"), "/home/user:/home/user/bin");

        // Edge cases
        assert_eq!(expand_shell_vars("literal $"), "literal $");
        assert_eq!(expand_shell_vars("no vars here"), "no vars here");
        assert_eq!(expand_shell_vars("~username/path"), "~username/path"); // ~user not supported

        if let Some(val) = old_test_var {
            env::set_var("TEST_VAR", val);
        } else {
            env::remove_var("TEST_VAR");
        }

        if let Some(val) = old_home {
            env::set_var("HOME", val);
        } else {
            env::remove_var("HOME");
        }

        if let Some(val) = old_user {
            env::set_var("USER", val);
        } else {
            env::remove_var("USER");
        }
    }

    #[test]
    fn test_source_with_env_vars() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_current_dir(temp_dir.path()).unwrap();
        env::set_var("WHI_SESSION_PID", "8888");
        env::set_var("PATH", "/usr/bin:/bin");
        env::set_var("TEST_EXPANSION", "expanded_value");
        env::set_var("USER", "testuser");
        env::remove_var("VIRTUAL_ENV_PROMPT");

        let whifile_content = "PATH!\n/usr/bin\n/bin\n/Users/$USER/.local/bin\n\nENV!\nRUST_LOG debug\nMY_VAR hello world\nEXPANDED $TEST_EXPANSION\n";
        fs::write(WHI_FILE, whifile_content).unwrap();

        let transition = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();

        // Check that PATH expansion worked
        assert_eq!(
            transition.new_path,
            "/usr/bin:/bin:/Users/testuser/.local/bin"
        );

        // Check that env vars are in env_changes (after venv bookkeeping vars)
        assert!(transition.env_changes.len() >= 6);
        assert!(transition.env_changes.iter().any(|change| matches!(
            change,
            EnvChange::Set(k, v) if k == "RUST_LOG" && v == "debug"
        )));
        assert!(transition.env_changes.iter().any(|change| matches!(
            change,
            EnvChange::Set(k, v) if k == "MY_VAR" && v == "hello world"
        )));

        // Check that variable expansion worked in ENV
        assert!(transition.env_changes.iter().any(|change| matches!(
            change,
            EnvChange::Set(k, v) if k == "EXPANDED" && v == "expanded_value"
        )));
        assert!(transition.env_changes.iter().any(|change| matches!(
            change,
            EnvChange::Set(k, v)
                if k == "WHI_VENV_DIR"
                    && *v == temp_dir
                        .path()
                        .to_string_lossy()
                        .to_string()
        )));

        // Set venv vars so exit_venv() knows we're in a venv
        env::set_var("VIRTUAL_ENV_PROMPT", "test");
        env::set_var("VIRTUAL_ENV", temp_dir.path().to_str().unwrap());

        // Clean up for exit test
        let exit_transition = exit_venv().unwrap();

        // Check that env vars are in env_changes as Unset operations
        assert!(exit_transition
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Unset(k) if k == "RUST_LOG")));
        assert!(exit_transition
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Unset(k) if k == "MY_VAR")));
        assert!(exit_transition
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Unset(k) if k == "EXPANDED")));
        assert!(exit_transition
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Unset(k) if k == "VIRTUAL_ENV_PROMPT")));
        assert!(exit_transition
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Unset(k) if k == "VIRTUAL_ENV")));
        assert!(exit_transition
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Unset(k) if k == "WHI_VENV_DIR")));

        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");
        env::remove_var("WHI_SESSION_PID");
        env::remove_var("PATH");
        env::remove_var("TEST_EXPANSION");
        env::remove_var("USER");

        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_source_exit_command_runs_before_unsets() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_current_dir(temp_dir.path()).unwrap();
        env::set_var("WHI_SESSION_PID", "5555");
        env::set_var("PATH", "/usr/bin:/bin");
        env::remove_var("VIRTUAL_ENV_PROMPT");

        let script_path = temp_dir.path().join("activate-extra.sh");
        fs::write(&script_path, "# test script\n").unwrap();
        let script_path_str = script_path.to_string_lossy().to_string();

        let whifile_content = format!(
            "!path.replace\n/usr/bin\n\n!whi.extra\n$source {} cleanup_extra\n",
            script_path.display()
        );
        fs::write(WHI_FILE, whifile_content).unwrap();

        let transition = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();
        assert!(transition
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Source(path) if path == &script_path_str)));

        env::set_var("VIRTUAL_ENV_PROMPT", "test");
        env::set_var("VIRTUAL_ENV", temp_dir.path().to_str().unwrap());

        let exit_transition = exit_venv().unwrap();
        assert!(
            matches!(exit_transition.env_changes.first(), Some(EnvChange::Run(cmd)) if cmd == "cleanup_extra")
        );

        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");
        env::remove_var("WHI_SESSION_PID");
        env::remove_var("PATH");

        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_protected_env_vars_never_unset() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        // Set up session
        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_var("WHI_SESSION_PID", "9999");
        env::set_var("WHI_SHELL_INITIALIZED", "1");
        env::set_var("__WHI_BIN", "/usr/local/bin/whi");
        env::set_var("PATH", "/usr/bin:/bin");
        env::set_var("SAFE_TO_UNSET", "value");
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("WHI_VENV_DIR");

        // Test 1: env.replace should NOT unset protected vars
        let whi_file = temp_dir.path().join("whifile");
        let content = r#"!path.replace
/new/path

!env.replace
SAFE_VAR safe_value
"#;
        fs::write(&whi_file, content).unwrap();

        let transition = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();

        // Verify protected vars are NOT in env_changes as Unset (they're protected from env.replace)
        let protected_vars_to_test = [
            "WHI_SHELL_INITIALIZED",
            "WHI_SESSION_PID",
            "__WHI_BIN",
            "PATH",
            "HOME",
            "USER",
            "SHELL",
            "TERM",
        ];

        for var in protected_vars_to_test {
            assert!(
                !transition
                    .env_changes
                    .iter()
                    .any(|change| matches!(change, EnvChange::Unset(k) if k == var)),
                "{} should never be unset by env.replace (it's protected)",
                var
            );
        }

        // Verify non-protected vars CAN be unset by env.replace
        assert!(
            transition
                .env_changes
                .iter()
                .any(|change| matches!(change, EnvChange::Unset(k) if k == "SAFE_TO_UNSET")),
            "Non-protected vars should be unset by env.replace"
        );

        // Clean up for test 2
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");
        env::remove_var("WHI_VENV_DIR");

        // Test 2: explicit env.unset SHOULD be able to unset protected vars
        let content2 = r#"!path.replace
/new/path

!env.unset
TERM
SAFE_TO_UNSET
"#;
        fs::write(&whi_file, content2).unwrap();

        let transition2 = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();

        // Verify that explicit unset works even for protected vars
        assert!(
            transition2
                .env_changes
                .iter()
                .any(|change| matches!(change, EnvChange::Unset(k) if k == "TERM")),
            "Explicit env.unset should work even for protected vars"
        );
        assert!(
            transition2
                .env_changes
                .iter()
                .any(|change| matches!(change, EnvChange::Unset(k) if k == "SAFE_TO_UNSET")),
            "Explicit env.unset should work for non-protected vars"
        );

        // Clean up
        env::remove_var("WHI_SESSION_PID");
        env::remove_var("WHI_SHELL_INITIALIZED");
        env::remove_var("__WHI_BIN");
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");
        env::remove_var("WHI_VENV_DIR");
        env::remove_var("WHI_VENV_DIR");
        env::remove_var("SAFE_TO_UNSET");

        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }

    #[test]
    fn test_env_operation_ordering() {
        let _guard = env_guard();
        let temp_dir = TempDir::new().unwrap();
        let xdg_before = env::var("XDG_RUNTIME_DIR").ok();

        // Set up session
        env::set_var("XDG_RUNTIME_DIR", temp_dir.path());
        env::set_var("WHI_SESSION_PID", "11111");
        env::set_var("PATH", "/usr/bin:/bin");
        env::set_var("EXISTING_VAR", "initial_value");
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("WHI_VENV_DIR");

        // Test 1: Unset followed by Set for same key should result in Set (var exists)
        let whi_file = temp_dir.path().join("whifile");
        let content = r#"!path.replace
/new/path

!env.unset
TEST_VAR

!env.set
TEST_VAR final_value
"#;
        fs::write(&whi_file, content).unwrap();

        let transition = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();

        // Find the position of unset and set operations in env_changes
        let mut unset_pos = None;
        let mut set_pos = None;

        for (i, change) in transition.env_changes.iter().enumerate() {
            match change {
                EnvChange::Unset(k) if k == "TEST_VAR" => unset_pos = Some(i),
                EnvChange::Set(k, v) if k == "TEST_VAR" && v == "final_value" => set_pos = Some(i),
                _ => {}
            }
        }

        // Verify both operations exist and Unset comes before Set
        assert!(unset_pos.is_some(), "Unset operation should exist");
        assert!(set_pos.is_some(), "Set operation should exist");
        assert!(
            unset_pos.unwrap() < set_pos.unwrap(),
            "Unset should come before Set to preserve ordering"
        );

        // Clean up for test 2
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");

        // Test 2: Set before Replace should result in Replace (simulated env state matters)
        let content2 = r#"!path.replace
/new/path

!env.set
WILL_BE_REPLACED keep_me

!env.replace
FINAL_VAR final_value
"#;
        fs::write(&whi_file, content2).unwrap();

        let transition2 = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();

        // After the Replace operation, WILL_BE_REPLACED should be unset
        // (because it's not in the replace list and it was added to simulated env by the Set)
        let has_will_be_replaced_set = transition2.env_changes.iter().any(
            |change| matches!(change, EnvChange::Set(k, v) if k == "WILL_BE_REPLACED" && v == "keep_me")
        );
        let has_will_be_replaced_unset = transition2
            .env_changes
            .iter()
            .any(|change| matches!(change, EnvChange::Unset(k) if k == "WILL_BE_REPLACED"));

        assert!(has_will_be_replaced_set, "SET should exist first");
        assert!(
            has_will_be_replaced_unset,
            "UNSET should exist after Replace (because Set added it to simulated env)"
        );

        // Verify the Set comes before the Unset (follows operation order)
        let set_pos = transition2
            .env_changes
            .iter()
            .position(|change| matches!(change, EnvChange::Set(k, _) if k == "WILL_BE_REPLACED"));
        let unset_pos = transition2
            .env_changes
            .iter()
            .position(|change| matches!(change, EnvChange::Unset(k) if k == "WILL_BE_REPLACED"));

        assert!(
            set_pos.unwrap() < unset_pos.unwrap(),
            "Set should come before Unset (preserving operation order)"
        );

        // Clean up for test 3
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");

        // Test 3: Complex ordering with multiple operations on same var
        let content3 = r#"!path.replace
/new/path

!env.set
FOO value1

!env.set
FOO value2

!env.unset
FOO

!env.set
FOO value3
"#;
        fs::write(&whi_file, content3).unwrap();

        let transition3 = source_from_path(temp_dir.path().to_str().unwrap()).unwrap();

        // Collect all operations on FOO in order
        let foo_operations: Vec<_> = transition3
            .env_changes
            .iter()
            .filter(|change| match change {
                EnvChange::Set(k, _) | EnvChange::Unset(k) => k == "FOO",
                EnvChange::Source(_) | EnvChange::Run(_) => false,
            })
            .collect();

        assert_eq!(foo_operations.len(), 4, "Should have 4 operations on FOO");

        // Verify the exact order
        assert!(
            matches!(foo_operations[0], EnvChange::Set(k, v) if k == "FOO" && v == "value1"),
            "First operation should be Set to value1"
        );
        assert!(
            matches!(foo_operations[1], EnvChange::Set(k, v) if k == "FOO" && v == "value2"),
            "Second operation should be Set to value2"
        );
        assert!(
            matches!(foo_operations[2], EnvChange::Unset(k) if k == "FOO"),
            "Third operation should be Unset"
        );
        assert!(
            matches!(foo_operations[3], EnvChange::Set(k, v) if k == "FOO" && v == "value3"),
            "Fourth operation should be Set to value3"
        );

        // Clean up
        env::remove_var("WHI_SESSION_PID");
        env::remove_var("VIRTUAL_ENV_PROMPT");
        env::remove_var("VIRTUAL_ENV");
        env::remove_var("EXISTING_VAR");

        if let Some(val) = xdg_before {
            env::set_var("XDG_RUNTIME_DIR", val);
        } else {
            env::remove_var("XDG_RUNTIME_DIR");
        }
    }
}
