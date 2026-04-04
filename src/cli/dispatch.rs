use std::process;

use clap::{Args as ClapArgs, CommandFactory, Parser, Subcommand, ValueEnum};

use crate::cli::{self, Args as AppArgs, ColorWhen};
use crate::commands;
use crate::config::{protected_paths, runtime, shell_paths};
use crate::path::file::{apply_path_sections, expand_shell_vars};
use crate::path::guard::PathGuard;
use crate::path::resolve::resolve_path;
use crate::path::PathSearcher;
use crate::session::history::HistoryContext;
use crate::session::store;
use crate::shell::detect::Shell;

#[derive(Parser, Debug)]
#[command(
    name = "whi",
    about = "PATH query utility backing whi shell functions",
    version,
    disable_help_subcommand = true,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
struct Cli {
    #[command(flatten)]
    query: QueryArgs,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(ClapArgs, Debug, Default)]
#[allow(clippy::struct_excessive_bools)]
struct QueryArgs {
    #[arg(short = 'a', long = "all")]
    all: bool,

    #[arg(short = 'f', long = "full")]
    full: bool,

    #[arg(short = 'l', long = "follow-symlinks", visible_alias = "L")]
    follow_symlinks: bool,

    #[arg(short = '0', long = "print0")]
    print0: bool,

    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    #[arg(long = "silent")]
    silent: bool,

    #[arg(short = '1', long = "one")]
    one: bool,

    #[arg(long = "show-nonexec", alias = "nonexec")]
    show_nonexec: bool,

    #[arg(long = "path")]
    path_override: Option<String>,

    #[arg(long = "color")]
    color: Option<ColorChoice>,

    #[arg(short = 's', long = "stat")]
    stat: bool,

    #[arg(short = 'n', long = "no-index")]
    no_index: bool,

    #[arg(short = 'x', long = "swap-fuzzy-exact")]
    swap_fuzzy: bool,

    #[arg(value_name = "NAME")]
    names: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show `PATH` changes since session start
    #[command(visible_alias = "d")]
    Diff(DiffArgs),
    /// Save current `PATH` to shell config files
    Apply(ApplyArgs),
    /// Print help message
    Help,
    /// Make an executable win by path, index, or pattern
    Prefer,
    /// Move a `PATH` entry to a different position
    Move,
    /// Swap two `PATH` entries
    Switch,
    /// Remove duplicate `PATH` entries
    Clean,
    /// Delete `PATH` entries by index, path, or pattern
    Delete,
    /// Reset `PATH` to initial session state
    Reset,
    /// Undo last `PATH` operation(s)
    Undo(UndoArgs),
    /// Redo next `PATH` operation(s)
    Redo(UndoArgs),
    /// Save current `PATH` as a named profile
    Save(SaveProfileArgs),
    /// Load a saved `PATH` profile
    Load(LoadProfileArgs),
    /// List all saved profiles
    List,
    /// Remove a saved profile
    #[command(name = "rmp")]
    RemoveProfile(RemoveProfileArgs),
    /// Add paths to `PATH` (prepends by default)
    Add,
    /// Show all whi shorthand commands
    Shorthands,
    #[command(hide = true)]
    Init(InitArgs),
    #[command(name = "__move", hide = true)]
    HiddenMove(HiddenMoveArgs),
    #[command(name = "__switch", hide = true)]
    HiddenSwap(HiddenSwapArgs),
    #[command(name = "__clean", hide = true)]
    HiddenClean,
    #[command(name = "__delete", hide = true)]
    HiddenDelete(HiddenDeleteArgs),
    #[command(name = "__prefer", hide = true)]
    HiddenPrefer(HiddenPreferArgs),
    #[command(name = "__reset", hide = true)]
    HiddenReset,
    #[command(name = "__undo", hide = true)]
    HiddenUndo(HiddenUndoArgs),
    #[command(name = "__redo", hide = true)]
    HiddenRedo(HiddenRedoArgs),
    #[command(name = "__load", hide = true)]
    HiddenLoad(HiddenLoadArgs),
    #[command(name = "__init", hide = true)]
    HiddenInit(HiddenInitArgs),
    #[command(name = "__load_saved_path", hide = true)]
    HiddenLoadSavedPath(HiddenLoadSavedPathArgs),
    #[command(name = "__add", hide = true)]
    HiddenAdd(HiddenAddArgs),
}

#[derive(ClapArgs, Debug, Default)]
struct DiffArgs {
    #[arg(value_name = "SHELL")]
    shell: Option<String>,

    /// Show unchanged entries in addition to changes
    #[arg(long = "full")]
    full: bool,
}

#[derive(ClapArgs, Debug, Default)]
struct ApplyArgs {
    #[arg(value_name = "SHELL")]
    shell: Option<String>,
    /// Skip protected paths (apply minimal `PATH` without safety)
    #[arg(long = "no-protect")]
    no_protect: bool,
}

#[derive(ClapArgs, Debug, Default)]
struct UndoArgs {
    #[arg(value_name = "COUNT", default_value = "1")]
    count: usize,
}

#[derive(ClapArgs, Debug)]
struct SaveProfileArgs {
    #[arg(value_name = "NAME", required = true)]
    name: String,
}

#[derive(ClapArgs, Debug)]
struct LoadProfileArgs {
    #[arg(value_name = "NAME", required = true)]
    name: String,
}

#[derive(ClapArgs, Debug)]
struct RemoveProfileArgs {
    #[arg(value_name = "NAME", required = true)]
    name: String,
}

#[derive(ClapArgs, Debug)]
struct HiddenUndoArgs {
    #[arg(value_name = "COUNT", default_value = "1")]
    count: usize,
}

#[derive(ClapArgs, Debug)]
struct HiddenRedoArgs {
    #[arg(value_name = "COUNT", default_value = "1")]
    count: usize,
}

#[derive(ClapArgs, Debug)]
struct HiddenLoadArgs {
    #[arg(value_name = "NAME", required = true)]
    name: String,
}

#[derive(ClapArgs, Debug)]
struct InitArgs {
    #[arg(value_name = "SHELL")]
    shell: String,
}

#[derive(ClapArgs, Debug)]
struct HiddenMoveArgs {
    #[arg(value_name = "FROM")]
    from: usize,
    #[arg(value_name = "TO")]
    to: usize,
}

#[derive(ClapArgs, Debug)]
struct HiddenSwapArgs {
    #[arg(value_name = "FIRST")]
    first: usize,
    #[arg(value_name = "SECOND")]
    second: usize,
}

#[derive(ClapArgs, Debug)]
struct HiddenDeleteArgs {
    #[arg(value_name = "TARGET", required = true)]
    targets: Vec<String>,
}

#[derive(ClapArgs, Debug)]
struct HiddenPreferArgs {
    #[arg(value_name = "ARGS", required = true)]
    tokens: Vec<String>,
}

#[derive(ClapArgs, Debug)]
struct HiddenInitArgs {
    #[arg(value_name = "PID", required = true)]
    session_pid: u32,
}

#[derive(ClapArgs, Debug)]
struct HiddenLoadSavedPathArgs {
    #[arg(value_name = "SHELL", required = true)]
    shell: String,
}

#[derive(ClapArgs, Debug)]
struct HiddenAddArgs {
    /// Paths to add to `PATH`
    #[arg(value_name = "PATH", required = true)]
    paths: Vec<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum ColorChoice {
    Auto,
    Never,
    Always,
}

impl From<ColorChoice> for ColorWhen {
    fn from(value: ColorChoice) -> ColorWhen {
        match value {
            ColorChoice::Auto => ColorWhen::Auto,
            ColorChoice::Never => ColorWhen::Never,
            ColorChoice::Always => ColorWhen::Always,
        }
    }
}

pub fn run() -> i32 {
    let cli_result = Cli::try_parse();

    // If parsing failed, rewrite error messages to hide internal command names
    let Cli { query, command } = match cli_result {
        Ok(cli) => cli,
        Err(err) => {
            let err_msg = err.to_string();

            // Rewrite hidden command names to their public equivalents
            let rewritten = err_msg
                .replace("whi __move", "whi move")
                .replace("whi __switch", "whi switch")
                .replace("whi __clean", "whi clean")
                .replace("whi __delete", "whi delete")
                .replace("whi __prefer", "whi prefer")
                .replace("whi __reset", "whi reset")
                .replace("whi __undo", "whi undo")
                .replace("whi __redo", "whi redo")
                .replace("whi __load", "whi load")
                .replace("whi __init", "whi init");

            // If the message was rewritten, print it and exit
            if rewritten != err_msg {
                eprint!("{rewritten}");
                process::exit(2);
            }

            // Otherwise, use the original error handling
            err.exit();
        }
    };

    if let Err(e) = runtime::ensure_config_exists() {
        eprintln!("Error: {e}");
        process::exit(2);
    }

    // Auto-migrate protected paths from config.toml to ~/.whi/protected_paths
    // This is a one-time migration that happens transparently on first run after upgrade
    if let Err(e) = protected_paths::migrate_from_config_toml() {
        eprintln!("Warning: Failed to migrate protected paths from config.toml: {e}");
        eprintln!("Your configuration may not have been fully migrated.");
        eprintln!("Please check ~/.whi/protected_paths and ~/.whi/config.toml");
    }

    // Ensure protected paths file exists with defaults so users can discover it
    if let Err(e) = protected_paths::ensure_protected_paths_exists() {
        eprintln!("Warning: Failed to create protected_paths file: {e}");
    }

    let exit_code = match command {
        Some(Command::Diff(diff)) => run_diff(diff),
        Some(Command::Apply(apply)) => run_apply(apply),
        Some(Command::Help) => run_help(),
        Some(
            Command::Prefer
            | Command::Move
            | Command::Switch
            | Command::Clean
            | Command::Delete
            | Command::Reset
            | Command::Undo(_)
            | Command::Redo(_)
            | Command::Load(_)
            | Command::Add,
        ) => check_shell_integration().unwrap_or(0),
        Some(Command::Save(save)) => run_save_profile(save),
        Some(Command::List) => run_list_profiles(),
        Some(Command::RemoveProfile(remove)) => run_remove_profile(remove),
        Some(Command::Init(init)) => run_init(init),
        Some(Command::HiddenMove(move_args)) => run_hidden_move(&move_args),
        Some(Command::HiddenSwap(swap_args)) => run_hidden_swap(&swap_args),
        Some(Command::HiddenClean) => run_hidden_clean(),
        Some(Command::HiddenDelete(delete_args)) => run_hidden_delete(delete_args),
        Some(Command::HiddenPrefer(prefer_args)) => run_hidden_prefer(prefer_args),
        Some(Command::HiddenReset) => run_hidden_reset(),
        Some(Command::HiddenUndo(undo_args)) => run_hidden_undo(&undo_args),
        Some(Command::HiddenRedo(redo_args)) => run_hidden_redo(&redo_args),
        Some(Command::HiddenLoad(load_args)) => run_hidden_load(&load_args),
        Some(Command::HiddenInit(args)) => run_hidden_init(&args),
        Some(Command::HiddenLoadSavedPath(args)) => run_hidden_load_saved_path(&args),
        Some(Command::HiddenAdd(add_args)) => run_hidden_add(&add_args),
        Some(Command::Shorthands) => run_shorthands(),
        None => run_query(query),
    };

    exit_code
}

/// Check if shell integration is loaded, return error code if not
fn check_shell_integration() -> Option<i32> {
    if std::env::var("WHI_SHELL_INITIALIZED").is_err() {
        eprintln!(
            "Shell integration not detected.\n\nRun one of these commands:\n  bash (current shell):    eval \"$(whi init bash)\"\n  bash (persistent):       add that line to the END of ~/.bashrc\n  zsh (current shell):     eval \"$(whi init zsh)\"\n  zsh (persistent):        add that line to the END of ~/.zshrc\n  fish (current shell):    whi init fish | source\n  fish (persistent):       add that line to the END of ~/.config/fish/config.fish\n"
        );
        return Some(2);
    }
    None
}

fn run_query(opts: QueryArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        names: opts.names,
        all: opts.all,
        full: opts.full,
        follow_symlinks: opts.follow_symlinks,
        print0: opts.print0,
        quiet: opts.quiet,
        silent: opts.silent,
        one: opts.one,
        show_nonexec: opts.show_nonexec,
        path_override: opts.path_override,
        color: opts.color.unwrap_or(ColorChoice::Auto).into(),
        stat: opts.stat,
        no_index: opts.no_index,
        swap_fuzzy: opts.swap_fuzzy,
        ..Default::default()
    };

    // Show usage only if no names AND no flags that imply listing PATH
    if args.names.is_empty() && !args.full && !args.all {
        println!(
            "Usage: whi [OPTIONS] [NAME]...\n       whi <COMMAND>\n\nTry 'whi --help' for more information."
        );
        return 0;
    }

    commands::run(&args)
}

fn run_diff(opts: DiffArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    // Check if "full" was passed as positional arg (legacy alias for --full)
    let full = match opts.shell {
        Some(shell) if shell.eq_ignore_ascii_case("full") => true,
        _ => opts.full,
    };

    let args = AppArgs {
        diff: true,
        diff_full: full,
        ..Default::default()
    };

    commands::run(&args)
}

fn run_apply(opts: ApplyArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        apply_shell: Some(opts.shell),
        no_protect: opts.no_protect,
        ..Default::default()
    };
    commands::run(&args)
}

fn run_save_profile(opts: SaveProfileArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        save_profile: Some(opts.name),
        ..Default::default()
    };
    commands::run(&args)
}

fn run_remove_profile(opts: RemoveProfileArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        remove_profile: Some(opts.name),
        ..Default::default()
    };
    commands::run(&args)
}

fn run_list_profiles() -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    match shell_paths::list_profiles() {
        Ok(profiles) => {
            if profiles.is_empty() {
                println!("No saved profiles");
            } else {
                for profile in profiles {
                    println!("{profile}");
                }
            }
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn run_init(opts: InitArgs) -> i32 {
    let args = AppArgs {
        init_shell: Some(opts.shell),
        ..Default::default()
    };
    commands::run(&args)
}

fn run_help() -> i32 {
    Cli::command().print_help().ok();
    println!();
    0
}

fn run_hidden_move(opts: &HiddenMoveArgs) -> i32 {
    let args = AppArgs {
        move_indices: Some((opts.from, opts.to)),
        ..Default::default()
    };
    commands::run(&args)
}

fn run_hidden_swap(opts: &HiddenSwapArgs) -> i32 {
    let args = AppArgs {
        swap_indices: Some((opts.first, opts.second)),
        ..Default::default()
    };
    commands::run(&args)
}

fn run_hidden_clean() -> i32 {
    let args = AppArgs {
        clean: true,
        ..Default::default()
    };
    commands::run(&args)
}

fn run_hidden_delete(opts: HiddenDeleteArgs) -> i32 {
    match cli::parse_delete_arguments(opts.targets) {
        Ok(targets) => {
            let args = AppArgs {
                delete_targets: targets,
                ..Default::default()
            };
            commands::run(&args)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            2
        }
    }
}

fn run_hidden_prefer(opts: HiddenPreferArgs) -> i32 {
    run_prefer_tokens(opts.tokens)
}

fn run_prefer_tokens(tokens: Vec<String>) -> i32 {
    match cli::parse_prefer_arguments(tokens) {
        Ok(target) => {
            let args = AppArgs {
                prefer_target: Some(target),
                ..Default::default()
            };
            commands::run(&args)
        }
        Err(err) => {
            eprintln!("Error: {err}");
            2
        }
    }
}

fn run_hidden_reset() -> i32 {
    let args = AppArgs {
        reset: true,
        ..Default::default()
    };
    commands::run(&args)
}

fn run_hidden_undo(opts: &HiddenUndoArgs) -> i32 {
    let args = AppArgs {
        undo_count: Some(opts.count),
        ..Default::default()
    };
    commands::run(&args)
}

fn run_hidden_redo(opts: &HiddenRedoArgs) -> i32 {
    let args = AppArgs {
        redo_count: Some(opts.count),
        ..Default::default()
    };
    commands::run(&args)
}

fn run_hidden_load(opts: &HiddenLoadArgs) -> i32 {
    use std::env;

    let session_pid = env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id);

    match shell_paths::load_profile(&opts.name) {
        Ok(parsed) => {
            let current_path = env::var("PATH").unwrap_or_default();
            let computed_path = match apply_path_sections(&current_path, &parsed.path) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Error applying profile: {e}");
                    return 2;
                }
            };

            let expanded_path = computed_path
                .split(':')
                .map(expand_shell_vars)
                .collect::<Vec<_>>()
                .join(":");

            if let Ok(history) = HistoryContext::global(session_pid) {
                let _ = history.write_snapshot(&expanded_path);
            }

            let guarded_path =
                PathGuard::default().ensure_protected_paths(&current_path, expanded_path);

            println!("{guarded_path}");
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn run_hidden_init(args: &HiddenInitArgs) -> i32 {
    use std::env;

    let path_var = env::var("PATH").unwrap_or_default();
    let session_pid = args.session_pid;

    match HistoryContext::global(session_pid) {
        Ok(history) => {
            if let Err(e) = history.reset_with_initial(&path_var) {
                eprintln!("Error: Failed to initialize session: {e}");
                return 2;
            }

            let _ = store::cleanup_old_sessions();

            0
        }
        Err(e) => {
            eprintln!("Error: Failed to prepare session history: {e}");
            2
        }
    }
}

fn run_hidden_load_saved_path(args: &HiddenLoadSavedPathArgs) -> i32 {
    use std::str::FromStr;

    let shell = match Shell::from_str(&args.shell) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            return 2;
        }
    };

    match shell_paths::load_saved_path_for_shell(&shell) {
        Ok(path) => {
            // Apply path guard to preserve critical binaries (whi, zoxide)
            let current_path = std::env::var("PATH").unwrap_or_default();
            let guarded_path = PathGuard::default().ensure_protected_paths(&current_path, path);

            println!("{guarded_path}");
            0
        }
        Err(e) => {
            eprintln!("Error loading saved PATH: {e}");
            1
        }
    }
}

fn run_hidden_add(args: &HiddenAddArgs) -> i32 {
    use std::env;
    use std::path::PathBuf;

    let session_pid = env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id);

    // Parse paths from arguments
    let paths = match cli::parse_add_arguments(args.paths.clone()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 2;
        }
    };

    // Get current PATH and create searcher once
    let current_path = env::var("PATH").unwrap_or_default();
    let mut searcher = PathSearcher::new(&current_path);

    // Resolve and add each path (prepend if not already in PATH)
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for path_str in paths {
        let resolved = match resolve_path(&path_str, &cwd) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Warning: Could not resolve path '{path_str}': {e}");
                // Try to use it as-is
                PathBuf::from(path_str)
            }
        };

        // Check if path is already in current PATH (deduplicate)
        if searcher.contains(&resolved) {
            continue; // Skip duplicates
        }

        // Prepend to PATH (add at index 1, which becomes the new first entry)
        if let Err(e) = searcher.insert_at(&resolved, 1) {
            eprintln!("Warning: Could not add '{}': {}", resolved.display(), e);
        }
    }

    let new_path = searcher.to_path_string();

    if let Ok(history) = HistoryContext::global(session_pid) {
        let _ = history.write_snapshot(&new_path);
    }

    // Apply path guard to preserve critical binaries (whi, zoxide)
    let guarded_path = PathGuard::default().ensure_protected_paths(&current_path, new_path);

    // Print raw PATH so shell helper can export it directly
    println!("{guarded_path}");
    0
}

struct Shorthand {
    name: &'static str,
    command: &'static str,
    description: &'static str,
}

const SHORTHANDS: &[Shorthand] = &[
    Shorthand {
        name: "whip",
        command: "whi prefer",
        description: "Make an executable win",
    },
    Shorthand {
        name: "whim",
        command: "whi move",
        description: "Move a PATH entry",
    },
    Shorthand {
        name: "whis",
        command: "whi switch",
        description: "Swap two PATH entries",
    },
    Shorthand {
        name: "whic",
        command: "whi clean",
        description: "Remove duplicates",
    },
    Shorthand {
        name: "whid",
        command: "whi delete",
        description: "Delete PATH entries",
    },
    Shorthand {
        name: "whia",
        command: "whi --all",
        description: "Show all matches",
    },
    Shorthand {
        name: "whiad",
        command: "whi add",
        description: "Add paths to PATH",
    },
    Shorthand {
        name: "whiu",
        command: "whi undo",
        description: "Undo last operation",
    },
    Shorthand {
        name: "whir",
        command: "whi redo",
        description: "Redo next operation",
    },
    Shorthand {
        name: "whil",
        command: "whi load",
        description: "Load saved profile",
    },
    Shorthand {
        name: "whish",
        command: "whi shorthands",
        description: "Show all shortcuts",
    },
];

fn run_shorthands() -> i32 {
    println!("Whi Shorthands:");

    for shorthand in SHORTHANDS {
        println!(
            "  {:<6} → {:<14} {}",
            shorthand.name, shorthand.command, shorthand.description
        );
    }
    println!();

    0
}
