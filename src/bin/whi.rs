use std::process;

use clap::{Args as ClapArgs, CommandFactory, Parser, Subcommand, ValueEnum};

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

    #[arg(value_name = "NAME")]
    names: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Show PATH changes since session start
    #[command(visible_alias = "d")]
    Diff(DiffArgs),
    /// Save current PATH to shell config files
    Apply(ApplyArgs),
    /// Print help message
    Help,
    /// Make an executable win by path, index, or pattern
    Prefer,
    /// Move a PATH entry to a different position
    Move,
    /// Swap two PATH entries
    Switch,
    /// Remove duplicate PATH entries
    Clean,
    /// Delete PATH entries by index, path, or pattern
    Delete,
    /// Reset PATH to initial session state
    Reset,
    /// Undo last PATH operation(s)
    Undo(UndoArgs),
    /// Redo next PATH operation(s)
    Redo(UndoArgs),
    /// Save current PATH as a named profile
    Save(SaveProfileArgs),
    /// Load a saved PATH profile
    Load(LoadProfileArgs),
    /// List all saved profiles
    List,
    /// Remove a saved profile
    #[command(name = "rmp")]
    RemoveProfile(RemoveProfileArgs),
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
            | Command::Load(_),
        ) => 0,
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
        Some(Command::HiddenLoad(load_args)) => run_hidden_load(load_args),
        Some(Command::HiddenInit(args)) => run_hidden_init(&args),
        None => run_query(query),
    };

    process::exit(exit_code);
}

fn run_query(opts: QueryArgs) -> i32 {
    if std::env::var("WHI_SHELL_INITIALIZED").is_err() {
        eprintln!("Shell integration not detected.\n\nRun one of these commands:\n  bash/zsh (current shell):   eval \"$(whi init bash)\"\n  bash/zsh (persistent):      add that line to the END of ~/.bashrc or ~/.zshrc\n  fish (current shell):       whi init fish | source\n  fish (persistent):          add that line to the END of ~/.config/fish/config.fish\n");
        return 2;
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
    let args = AppArgs {
        apply_shell: Some(opts.shell),
        ..Default::default()
    };
    app::run(&args)
}

fn run_save_profile(opts: SaveProfileArgs) -> i32 {
    let args = AppArgs {
        save_profile: Some(opts.name),
        ..Default::default()
    };
    app::run(&args)
}

fn run_remove_profile(opts: RemoveProfileArgs) -> i32 {
    let args = AppArgs {
        remove_profile: Some(opts.name),
        ..Default::default()
    };
    app::run(&args)
}

fn run_list_profiles() -> i32 {
    use whi::config_manager::list_profiles;

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

fn run_hidden_load(opts: HiddenLoadArgs) -> i32 {
    let args = AppArgs {
        load_profile: Some(opts.name),
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_init(args: &HiddenInitArgs) -> i32 {
    use std::env;
    use whi::session_tracker;

    let path_var = env::var("PATH").unwrap_or_default();
    let session_pid = args.session_pid;

    // Clear any existing session data (new shell = fresh start)
    if let Err(e) = session_tracker::clear_session(session_pid) {
        eprintln!("Warning: Failed to clear old session: {e}");
    }

    // Write initial snapshot
    match session_tracker::write_path_snapshot(session_pid, &path_var) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Error: Failed to initialize session: {e}");
            2
        }
    }
}
