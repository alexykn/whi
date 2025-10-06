use std::process;

use clap::{Args as ClapArgs, CommandFactory, Parser, Subcommand, ValueEnum};
use whi::config_manager::list_profiles;
use whi::venv_manager;

use whi::app;
use whi::cli::{self, Args as AppArgs, ColorWhen};

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
    /// Create whifile from current `PATH`
    File(FileArgs),
    /// Add paths to `PATH` (prepends by default)
    Add,
    /// Query environment variables
    Var(VarArgs),
    /// Show all whi shorthand commands
    Shorthands,
    /// Activate venv from whifile
    Source,
    /// Exit active venv
    Exit,
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
    #[command(name = "__should_auto_activate", hide = true)]
    HiddenShouldAutoActivate,
    #[command(name = "__venv_source", hide = true)]
    HiddenVenvSource(HiddenVenvSourceArgs),
    #[command(name = "__venv_exit", hide = true)]
    HiddenVenvExit,
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
    /// Apply even if a venv is currently active
    #[arg(short = 'f', long = "force")]
    force: bool,
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
struct HiddenVenvSourceArgs {
    #[arg(value_name = "PATH", required = true)]
    path: String,
}

#[derive(ClapArgs, Debug)]
struct HiddenLoadSavedPathArgs {
    #[arg(value_name = "SHELL", required = true)]
    shell: String,
}

#[derive(Clone, Copy, ClapArgs, Debug, Default)]
struct FileArgs {
    /// Force overwriting existing whifile with current `PATH`
    #[arg(short = 'f', long = "force")]
    force: bool,
}

#[derive(ClapArgs, Debug)]
struct HiddenAddArgs {
    /// Paths to add to `PATH`
    #[arg(value_name = "PATH", required = true)]
    paths: Vec<String>,
}

#[derive(ClapArgs, Debug)]
struct VarArgs {
    /// List all environment variables
    #[arg(short = 'f', long = "full")]
    full: bool,

    /// Swap fuzzy search method (invert config setting)
    #[arg(short = 'x', long = "swap-fuzzy-exact")]
    swap_fuzzy: bool,

    /// Output only value (no key), like echo $VAR
    #[arg(short = 'n', long = "no-key")]
    no_key: bool,

    /// Variable name or fuzzy pattern to search for
    #[arg(value_name = "NAME")]
    query: Option<String>,
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

fn main() {
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

    if let Err(e) = whi::config::ensure_config_exists() {
        eprintln!("Error: {e}");
        process::exit(2);
    }

    // Auto-migrate protected paths from config.toml to ~/.whi/protected_paths
    // This is a one-time migration that happens transparently on first run after upgrade
    if let Err(e) = whi::protected_config::migrate_from_config_toml() {
        eprintln!("Warning: Failed to migrate protected paths from config.toml: {e}");
        eprintln!("Your configuration may not have been fully migrated.");
        eprintln!("Please check ~/.whi/protected_paths and ~/.whi/config.toml");
    }

    // Ensure protected paths and vars files exist with defaults so users can discover them
    if let Err(e) = whi::protected_config::ensure_protected_paths_exists() {
        eprintln!("Warning: Failed to create protected_paths file: {e}");
    }
    if let Err(e) = whi::protected_config::ensure_protected_vars_exists() {
        eprintln!("Warning: Failed to create protected_vars file: {e}");
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
            | Command::Add
            | Command::Source
            | Command::Exit,
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
        Some(Command::File(file_args)) => run_file(file_args),
        Some(Command::HiddenShouldAutoActivate) => run_should_auto_activate(),
        Some(Command::HiddenVenvSource(args)) => run_hidden_venv_source(&args),
        Some(Command::HiddenVenvExit) => run_hidden_venv_exit(),
        Some(Command::HiddenLoadSavedPath(args)) => run_hidden_load_saved_path(&args),
        Some(Command::HiddenAdd(add_args)) => run_hidden_add(&add_args),
        Some(Command::Var(var_args)) => run_var(&var_args),
        Some(Command::Shorthands) => run_shorthands(),
        None => run_query(query),
    };

    process::exit(exit_code);
}

/// Check if shell integration is loaded, return error code if not
fn check_shell_integration() -> Option<i32> {
    if std::env::var("WHI_SHELL_INITIALIZED").is_err() {
        eprintln!("Shell integration not detected.\n\nRun one of these commands:\n  bash (current shell):    eval \"$(whi init bash)\"\n  bash (persistent):       add that line to the END of ~/.bashrc\n  zsh (current shell):     eval \"$(whi init zsh)\"\n  zsh (persistent):        add that line to the END of ~/.zshrc\n  fish (current shell):    whi init fish | source\n  fish (persistent):       add that line to the END of ~/.config/fish/config.fish\n");
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
        println!("Usage: whi [OPTIONS] [NAME]...\n       whi <COMMAND>\n\nTry 'whi --help' for more information.");
        return 0;
    }

    app::run(&args)
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

    app::run(&args)
}

fn run_apply(opts: ApplyArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        apply_shell: Some(opts.shell),
        apply_force: opts.force,
        no_protect: opts.no_protect,
        ..Default::default()
    };
    let exit_code = app::run(&args);

    if exit_code == 0 && whi::venv_manager::is_in_venv() {
        if let Ok(shell) = whi::shell_detect::detect_current_shell() {
            if let Ok(saved_path) = whi::config_manager::load_saved_path_for_shell(&shell) {
                if let Err(e) = whi::venv_manager::update_restore_path(&saved_path) {
                    eprintln!("Warning: Failed to update session PATH: {e}");
                }
            }
        }
    }

    exit_code
}

fn run_save_profile(opts: SaveProfileArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        save_profile: Some(opts.name),
        ..Default::default()
    };
    app::run(&args)
}

fn run_remove_profile(opts: RemoveProfileArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        remove_profile: Some(opts.name),
        ..Default::default()
    };
    app::run(&args)
}

fn run_list_profiles() -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    match list_profiles() {
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
    app::run(&args)
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
    app::run(&args)
}

fn run_hidden_swap(opts: &HiddenSwapArgs) -> i32 {
    let args = AppArgs {
        swap_indices: Some((opts.first, opts.second)),
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_clean() -> i32 {
    let args = AppArgs {
        clean: true,
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_delete(opts: HiddenDeleteArgs) -> i32 {
    match cli::parse_delete_arguments(opts.targets) {
        Ok(targets) => {
            let args = AppArgs {
                delete_targets: targets,
                ..Default::default()
            };
            app::run(&args)
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
            app::run(&args)
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
    app::run(&args)
}

fn run_hidden_undo(opts: &HiddenUndoArgs) -> i32 {
    let args = AppArgs {
        undo_count: Some(opts.count),
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_redo(opts: &HiddenRedoArgs) -> i32 {
    let args = AppArgs {
        redo_count: Some(opts.count),
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_load(opts: &HiddenLoadArgs) -> i32 {
    use std::env;
    use whi::config_manager::load_profile;
    use whi::history::HistoryContext;
    use whi::path_file::{apply_path_sections, EnvOperation};

    let session_pid = env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id);

    match load_profile(&opts.name) {
        Ok(parsed) => {
            // Get current PATH to use as base for prepend/append
            let current_path = env::var("PATH").unwrap_or_default();

            // Apply PATH sections
            let computed_path = match apply_path_sections(&current_path, &parsed.path) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Error applying profile: {e}");
                    return 2;
                }
            };

            // Expand shell variables in computed PATH entries
            let expanded_path = computed_path
                .split(':')
                .map(whi::venv_manager::expand_shell_vars)
                .collect::<Vec<_>>()
                .join(":");

            // Update history using whi-owned identifier when available
            if env::var("VIRTUAL_ENV_PROMPT").is_err() {
                if let Ok(history) = HistoryContext::global(session_pid) {
                    let _ = history.write_snapshot(&expanded_path);
                }
            } else if let Some(venv_dir) = whi::venv_manager::current_venv_dir() {
                if let Ok(history) = HistoryContext::venv(session_pid, venv_dir.as_path()) {
                    let _ = history.write_snapshot(&expanded_path);
                }
            } else if let Ok(history) = HistoryContext::global(session_pid) {
                // Fallback: missing metadata, keep session usable
                let _ = history.write_snapshot(&expanded_path);
            }

            // Apply path guard to preserve critical binaries (whi, zoxide)
            let guarded_path = whi::path_guard::PathGuard::default()
                .ensure_protected_paths(&current_path, expanded_path);

            // Print transition protocol
            println!("PATH\t{guarded_path}");

            // Handle env operations in order
            // Note: Profiles currently only support Set operations. Unset and Replace are not yet supported
            // because profiles are meant to save PATH states, not perform environment replacement.
            for operation in &parsed.env.operations {
                match operation {
                    EnvOperation::Set(key, value) => {
                        let expanded_value = whi::venv_manager::expand_shell_vars(value);
                        println!("SET\t{key}\t{expanded_value}");
                    }
                    EnvOperation::Unset(_) => {
                        eprintln!("Warning: !env.unset not yet supported for profiles, ignoring");
                    }
                    EnvOperation::Replace(_) => {
                        eprintln!("Warning: !env.replace not yet supported for profiles, ignoring");
                    }
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

fn run_hidden_init(args: &HiddenInitArgs) -> i32 {
    use std::env;
    use whi::history::{HistoryContext, HistoryScope};
    use whi::session_tracker;

    let path_var = env::var("PATH").unwrap_or_default();
    let session_pid = args.session_pid;

    match HistoryContext::global(session_pid) {
        Ok(history) => {
            if let Err(e) = history.reset_with_initial(&path_var) {
                eprintln!("Error: Failed to initialize session: {e}");
                return 2;
            }

            if history.scope() == HistoryScope::Global {
                let _ = session_tracker::cleanup_old_sessions();
            }

            0
        }
        Err(e) => {
            eprintln!("Error: Failed to prepare session history: {e}");
            2
        }
    }
}

fn run_file(opts: FileArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    match venv_manager::create_file(opts.force) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn run_hidden_venv_source(args: &HiddenVenvSourceArgs) -> i32 {
    use whi::venv_manager;

    match venv_manager::source_from_path(&args.path) {
        Ok(transition) => {
            print_venv_transition(&transition);
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn run_hidden_venv_exit() -> i32 {
    use whi::venv_manager;

    match venv_manager::exit_venv() {
        Ok(transition) => {
            print_venv_transition(&transition);
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn run_should_auto_activate() -> i32 {
    use whi::config::load_config;

    if let Ok(config) = load_config() {
        let file_val = i32::from(config.venv.auto_activate_file);
        let deactivate_val = i32::from(config.venv.auto_deactivate_file);
        println!("file={file_val}");
        println!("deactivate={deactivate_val}");
        0
    } else {
        // Default to false on error
        println!("file=0");
        println!("deactivate=0");
        0
    }
}

fn run_hidden_load_saved_path(args: &HiddenLoadSavedPathArgs) -> i32 {
    use std::str::FromStr;
    use whi::config_manager;
    use whi::shell_detect::Shell;

    let shell = match Shell::from_str(&args.shell) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            return 2;
        }
    };

    match config_manager::load_saved_path_for_shell(&shell) {
        Ok(path) => {
            // Apply path guard to preserve critical binaries (whi, zoxide)
            let current_path = std::env::var("PATH").unwrap_or_default();
            let guarded_path =
                whi::path_guard::PathGuard::default().ensure_protected_paths(&current_path, path);

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
    use whi::history::HistoryContext;
    use whi::path::PathSearcher;
    use whi::path_resolver::resolve_path;

    let session_pid = env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id);

    // Parse paths from arguments
    let paths = match whi::cli::parse_add_arguments(args.paths.clone()) {
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

    // Update history using whi-owned identifier when available
    if env::var("VIRTUAL_ENV_PROMPT").is_err() {
        if let Ok(history) = HistoryContext::global(session_pid) {
            let _ = history.write_snapshot(&new_path);
        }
    } else if let Some(venv_dir) = whi::venv_manager::current_venv_dir() {
        if let Ok(history) = HistoryContext::venv(session_pid, venv_dir.as_path()) {
            let _ = history.write_snapshot(&new_path);
        }
    } else if let Ok(history) = HistoryContext::global(session_pid) {
        let _ = history.write_snapshot(&new_path);
    }

    // Apply path guard to preserve critical binaries (whi, zoxide)
    let guarded_path =
        whi::path_guard::PathGuard::default().ensure_protected_paths(&current_path, new_path);

    // Print raw PATH so shell helper can export it directly
    println!("{guarded_path}");
    0
}

fn run_var(args: &VarArgs) -> i32 {
    use std::env;
    use whi::path_resolver::FuzzyMatcher;

    // Load config for fuzzy search settings
    let config = whi::config::load_config().unwrap_or_default();

    // Validate flags: -f should only be used without a query
    if args.full && args.query.is_some() {
        eprintln!("Error: -f/--full flag cannot be used with a variable name");
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  whi var -f             # List all variables");
        eprintln!("  whi var NAME           # Query specific variable");
        return 2;
    }

    // Handle -f/--full flag: list all environment variables
    if args.full {
        let mut vars: Vec<(String, String)> = env::vars().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));

        for (key, value) in vars {
            if args.no_key {
                println!("{value}");
            } else {
                println!("{key} {value}");
            }
        }
        return 0;
    }

    // If no query provided, show usage
    let Some(query) = &args.query else {
        eprintln!("Usage: whi var [-f|--full] [NAME]");
        eprintln!("  Query environment variables");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  -f, --full    List all environment variables (only valid without NAME)");
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  whi var PATH           # Show PATH variable");
        eprintln!("  whi var path           # Case-insensitive exact match");
        eprintln!("  whi var cargo          # Fuzzy search for variables matching 'cargo'");
        eprintln!("  whi var -f             # List all variables");
        return 2;
    };

    // Determine fuzzy mode: config XOR swap flag
    let use_fuzzy = config.search.variable_search_fuzzy ^ args.swap_fuzzy;

    if use_fuzzy {
        // Fuzzy search enabled: search directly with fuzzy, no exact check
        let matcher = FuzzyMatcher::new(query);
        let mut results: Vec<(String, String)> = env::vars()
            .filter(|(key, _)| {
                use std::path::Path;
                matcher.matches(Path::new(key))
            })
            .collect();

        if results.is_empty() {
            eprintln!("No environment variable matching '{query}' found");
            return 1;
        }

        // Sort results by key name
        results.sort_by(|a, b| a.0.cmp(&b.0));

        for (key, value) in results {
            if args.no_key {
                println!("{value}");
            } else {
                println!("{key} {value}");
            }
        }

        0
    } else {
        // Fuzzy disabled: exact match only (case-insensitive)
        let query_upper = query.to_uppercase();

        for (key, value) in env::vars() {
            if key.to_uppercase() == query_upper {
                if args.no_key {
                    println!("{value}");
                } else {
                    println!("{key} {value}");
                }
                return 0;
            }
        }

        // No exact match found
        eprintln!("No environment variable matching '{query}' found");
        1
    }
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
        name: "whiv",
        command: "whi var",
        description: "Query env variables",
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

fn print_venv_transition(transition: &whi::venv_manager::VenvTransition) {
    use whi::venv_manager::EnvChange;

    // CRITICAL: Deactivate Python venv BEFORE restoring PATH
    if transition.needs_pyenv_deactivate {
        println!("DEACTIVATE_PYENV");
    }

    println!("PATH\t{}", transition.new_path);

    // Print env changes in order
    for change in &transition.env_changes {
        match change {
            EnvChange::Set(key, value) => {
                println!("SET\t{key}\t{value}");
            }
            EnvChange::Unset(key) => {
                println!("UNSET\t{key}");
            }
            EnvChange::Source(path) => {
                println!("SOURCE\t{path}");
            }
            EnvChange::Run(command) => {
                println!("RUN\t{command}");
            }
        }
    }
}
