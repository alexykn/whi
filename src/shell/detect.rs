use std::env;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
        }
    }
}

impl FromStr for Shell {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bash" => Ok(Shell::Bash),
            "zsh" => Ok(Shell::Zsh),
            "fish" => Ok(Shell::Fish),
            _ => Err(format!(
                "Unknown shell: {s}. Valid options: bash, zsh, fish"
            )),
        }
    }
}

/// Detect the current shell from environment
pub fn detect_current_shell() -> Result<Shell, String> {
    if let Ok(shell_path) = env::var("SHELL") {
        if shell_path.contains("bash") {
            return Ok(Shell::Bash);
        } else if shell_path.contains("zsh") {
            return Ok(Shell::Zsh);
        } else if shell_path.contains("fish") {
            return Ok(Shell::Fish);
        }
    }

    if env::var("BASH_VERSION").is_ok() {
        return Ok(Shell::Bash);
    }
    if env::var("ZSH_VERSION").is_ok() {
        return Ok(Shell::Zsh);
    }
    if env::var("FISH_VERSION").is_ok() {
        return Ok(Shell::Fish);
    }

    Err("Could not detect shell. Please specify: bash, zsh, or fish".to_string())
}

/// Get the config file path for a given shell
pub fn get_config_file_path(shell: &Shell) -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?;
    let home_path = PathBuf::from(home);

    match shell {
        Shell::Bash => {
            let bash_profile = home_path.join(".bash_profile");
            if bash_profile.exists() {
                return Ok(bash_profile);
            }

            let bashrc = home_path.join(".bashrc");
            if bashrc.exists() {
                return Ok(bashrc);
            }

            #[cfg(target_os = "macos")]
            {
                Ok(bash_profile)
            }
            #[cfg(not(target_os = "macos"))]
            {
                Ok(bashrc)
            }
        }
        Shell::Zsh => Ok(home_path.join(".zprofile")),
        Shell::Fish => Ok(home_path.join(".config/fish/config.fish")),
    }
}

/// Get the path to the saved `PATH` file for a given shell
pub fn get_saved_path_file(shell: &Shell) -> Result<PathBuf, String> {
    let home = env::var("HOME").map_err(|_| "HOME environment variable not set".to_string())?;
    let whi_dir = PathBuf::from(home).join(".whi");

    Ok(whi_dir.join(format!("saved_path_{}", shell.as_str())))
}

/// Get the sourcing line to add to shell config
pub fn get_sourcing_line(shell: &Shell) -> Result<String, String> {
    let saved_path_file = get_saved_path_file(shell)?;
    let saved_path_str = saved_path_file.display().to_string();
    let shell_name = shell.as_str();

    match shell {
        Shell::Bash | Shell::Zsh => Ok(format!(
            "# whi: Load saved PATH\nif [ -f {saved_path_str} ]; then\n    NEW_PATH=$(whi __load_saved_path {shell_name} 2>/dev/null)\n    [ -n \"$NEW_PATH\" ] && export PATH=\"$NEW_PATH\"\nfi\n"
        )),
        Shell::Fish => Ok(format!(
            "# whi: Load saved PATH\nif test -f {saved_path_str}\n    set -l new_path (whi __load_saved_path {shell_name} 2>/dev/null)\n    if test -n \"$new_path\"\n        set -gx PATH (string split : -- $new_path)\n    end\nend\n"
        )),
    }
}
