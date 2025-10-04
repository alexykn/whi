#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorWhen {
    #[default]
    Auto,
    Never,
    Always,
}

#[derive(Debug, Clone)]
pub enum PreferTarget {
    /// Traditional index-based preference (backward compatible)
    IndexBased { name: String, index: usize },
    /// Path-based preference (new feature)
    PathBased { name: String, path: String },
    /// Path-only preference (like `fish_add_path`)
    PathOnly { path: String },
}

#[derive(Debug, Clone)]
pub enum DeleteTarget {
    /// Traditional index-based deletion
    Index(usize),
    /// Path-based deletion (exact or fuzzy)
    Path(String),
}

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Default, Clone)]
pub struct Args {
    pub names: Vec<String>,
    pub all: bool,
    pub full: bool,
    pub follow_symlinks: bool,
    pub print0: bool,
    pub quiet: bool,
    pub silent: bool,
    pub one: bool,
    pub show_nonexec: bool,
    pub path_override: Option<String>,
    pub color: ColorWhen,
    pub stat: bool,
    pub no_index: bool,
    pub swap_fuzzy: bool,
    pub move_indices: Option<(usize, usize)>,
    pub swap_indices: Option<(usize, usize)>,
    pub prefer_target: Option<PreferTarget>,
    pub clean: bool,
    pub delete_targets: Vec<DeleteTarget>,
    pub apply_shell: Option<Option<String>>, // None = not used, Some(None) = current, Some(Some(x)) = specific
    pub apply_force: bool,
    pub no_protect: bool,
    pub diff: bool,
    pub diff_full: bool,
    pub init_shell: Option<String>,
    pub reset: bool,
    pub undo_count: Option<usize>, // None = not used, Some(n) = undo n operations
    pub redo_count: Option<usize>, // None = not used, Some(n) = redo n operations
    pub save_profile: Option<String>,
    pub load_profile: Option<String>,
    pub remove_profile: Option<String>,
}

impl Args {
    pub fn parse_color(color: &str) -> Result<ColorWhen, String> {
        match color {
            "auto" => Ok(ColorWhen::Auto),
            "never" => Ok(ColorWhen::Never),
            "always" => Ok(ColorWhen::Always),
            other => Err(format!(
                "Invalid color option: {other}. Expected auto, never, or always"
            )),
        }
    }
}

/// Parse arguments for the hidden `__prefer` command.
pub fn parse_prefer_arguments(tokens: Vec<String>) -> Result<PreferTarget, String> {
    if tokens.is_empty() {
        return Err("prefer requires at least one argument".to_string());
    }

    if tokens.len() == 1 {
        let path = tokens.into_iter().next().unwrap();
        return Ok(PreferTarget::PathOnly { path });
    }

    let mut iter = tokens.into_iter();
    let name = iter.next().unwrap();
    let remaining: Vec<String> = iter.collect();
    let target_raw = remaining.join(" ");

    if remaining.len() == 1 {
        let candidate = &remaining[0];
        if let Ok(index) = candidate.parse::<usize>() {
            if !looks_like_path(candidate) {
                return Ok(PreferTarget::IndexBased { name, index });
            }
        }
    }

    Ok(PreferTarget::PathBased {
        name,
        path: target_raw,
    })
}

/// Parse arguments for the hidden `__delete` command.
pub fn parse_delete_arguments(tokens: Vec<String>) -> Result<Vec<DeleteTarget>, String> {
    if tokens.is_empty() {
        return Err("delete requires at least one target".to_string());
    }

    if tokens.iter().all(|t| {
        t.parse::<usize>()
            .ok()
            .filter(|_| !looks_like_path(t))
            .is_some()
    }) {
        let indices = tokens
            .into_iter()
            .map(|t| DeleteTarget::Index(t.parse::<usize>().unwrap()))
            .collect();
        return Ok(indices);
    }

    if tokens.len() == 1 {
        return Ok(vec![parse_delete_target(&tokens[0])]);
    }

    let joined = tokens.join(" ");
    Ok(vec![parse_delete_target(&joined)])
}

fn parse_delete_target(target_str: &str) -> DeleteTarget {
    if let Ok(index) = target_str.parse::<usize>() {
        if !looks_like_path(target_str) {
            return DeleteTarget::Index(index);
        }
    }

    DeleteTarget::Path(target_str.to_string())
}

fn looks_like_path(s: &str) -> bool {
    s.contains('/') || s.starts_with('~') || s.starts_with('.') || s.contains('\\')
}

/// Parse arguments for the hidden `__add` command.
pub fn parse_add_arguments(tokens: Vec<String>) -> Result<Vec<String>, String> {
    if tokens.is_empty() {
        return Err("add requires at least one path".to_string());
    }

    // Return paths as-is, preserving all provided paths
    Ok(tokens)
}
