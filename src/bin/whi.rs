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
    Diff(DiffArgs),
    Save(SaveArgs),
    Help,
    Prefer,
    Move,
    Switch,
    Clean,
    Delete,
    #[command(hide = true)]
    Init(InitArgs),
    #[command(name = "__move", hide = true)]
    HiddenMove(HiddenMoveArgs),
    #[command(name = "__swap", hide = true, visible_alias = "__switch")]
    HiddenSwap(HiddenSwapArgs),
    #[command(name = "__clean", hide = true)]
    HiddenClean,
    #[command(name = "__delete", hide = true)]
    HiddenDelete(HiddenDeleteArgs),
    #[command(name = "__prefer", hide = true)]
    HiddenPrefer(HiddenPreferArgs),
}

#[derive(ClapArgs, Debug, Default)]
struct DiffArgs {
    #[arg(value_name = "SHELL")]
    shell: Option<String>,

    #[arg(long = "full")]
    full: bool,
}

#[derive(ClapArgs, Debug, Default)]
struct SaveArgs {
    #[arg(value_name = "SHELL")]
    shell: Option<String>,
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
                .replace("whi __swap", "whi switch")
                .replace("whi __clean", "whi clean")
                .replace("whi __delete", "whi delete")
                .replace("whi __prefer", "whi prefer");

            // If the message was rewritten, print it and exit
            if rewritten != err_msg {
                eprint!("{}", rewritten);
                process::exit(2);
            }

            // Otherwise, use the original error handling
            err.exit();
        }
    };

    let exit_code = match command {
        Some(Command::Diff(diff)) => run_diff(diff),
        Some(Command::Save(save)) => run_save(save),
        Some(Command::Help) => run_help(),
        Some(Command::Prefer) => 0,
        Some(Command::Move) => 0,
        Some(Command::Switch) => 0,
        Some(Command::Clean) => 0,
        Some(Command::Delete) => 0,
        Some(Command::Init(init)) => run_init(init),
        Some(Command::HiddenMove(move_args)) => run_hidden_move(move_args),
        Some(Command::HiddenSwap(swap_args)) => run_hidden_swap(swap_args),
        Some(Command::HiddenClean) => run_hidden_clean(),
        Some(Command::HiddenDelete(delete_args)) => run_hidden_delete(delete_args),
        Some(Command::HiddenPrefer(prefer_args)) => run_hidden_prefer(prefer_args),
        None => run_query(query),
    };

    process::exit(exit_code);
}

fn run_query(opts: QueryArgs) -> i32 {
    if std::env::var("WHI_SHELL_INITIALIZED").is_err() {
        eprintln!("Shell integration not detected.\n\nRun one of these commands:\n  bash/zsh (current shell):   eval \"$(whi init bash)\"\n  bash/zsh (persistent):      add that line to ~/.bashrc or ~/.zshrc\n  fish (current shell):       whi init fish | source\n  fish (persistent):          add that line to ~/.config/fish/config.fish\n");
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
    let (diff_shell, diff_full) = match opts.shell {
        Some(shell) if shell.eq_ignore_ascii_case("full") => (Some(None), true),
        Some(shell) => (Some(Some(shell)), opts.full),
        None => (Some(None), opts.full),
    };

    let args = AppArgs {
        diff_shell,
        diff_full,
        ..Default::default()
    };

    app::run(&args)
}

fn run_save(opts: SaveArgs) -> i32 {
    let args = AppArgs {
        save_shell: Some(opts.shell),
        ..Default::default()
    };
    app::run(&args)
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

fn run_hidden_move(opts: HiddenMoveArgs) -> i32 {
    let args = AppArgs {
        requires_integration: true,
        move_indices: Some((opts.from, opts.to)),
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_swap(opts: HiddenSwapArgs) -> i32 {
    let args = AppArgs {
        requires_integration: true,
        swap_indices: Some((opts.first, opts.second)),
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_clean() -> i32 {
    let args = AppArgs {
        requires_integration: true,
        clean: true,
        ..Default::default()
    };
    app::run(&args)
}

fn run_hidden_delete(opts: HiddenDeleteArgs) -> i32 {
    match cli::parse_delete_arguments(opts.targets) {
        Ok(targets) => {
            let args = AppArgs {
                requires_integration: true,
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
                requires_integration: true,
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
