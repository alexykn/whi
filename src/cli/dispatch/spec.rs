use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};

use crate::cli::ColorWhen;

#[derive(Parser, Debug)]
#[command(
    name = "whi",
    about = "PATH query utility backing whi shell functions",
    version,
    disable_help_subcommand = true,
    args_conflicts_with_subcommands = true,
    subcommand_negates_reqs = true
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) query: QueryArgs,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct QueryArgs {
    #[command(flatten)]
    pub(crate) listing: QueryListingArgs,

    #[command(flatten)]
    pub(crate) listing_details: QueryListingDetailsArgs,

    #[command(flatten)]
    pub(crate) output: QueryOutputArgs,

    #[command(flatten)]
    pub(crate) output_details: QueryOutputDetailsArgs,

    #[command(flatten)]
    pub(crate) mode: QueryModeArgs,

    #[arg(long = "path")]
    pub(crate) path_override: Option<String>,

    #[arg(long = "color")]
    pub(crate) color: Option<ColorChoice>,

    #[arg(value_name = "NAME")]
    pub(crate) names: Vec<String>,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct QueryListingArgs {
    #[arg(short = 'a', long = "all")]
    pub(crate) all: bool,

    #[arg(short = 'f', long = "full")]
    pub(crate) full: bool,

    #[arg(short = 'l', long = "follow-symlinks", visible_alias = "L")]
    pub(crate) follow_symlinks: bool,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct QueryListingDetailsArgs {
    #[arg(short = '1', long = "one")]
    pub(crate) one: bool,

    #[arg(long = "show-nonexec", alias = "nonexec")]
    pub(crate) show_nonexec: bool,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct QueryOutputArgs {
    #[arg(short = '0', long = "print0")]
    pub(crate) print0: bool,

    #[arg(short = 'q', long = "quiet")]
    pub(crate) quiet: bool,

    #[arg(long = "silent")]
    pub(crate) silent: bool,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct QueryOutputDetailsArgs {
    #[arg(short = 's', long = "stat")]
    pub(crate) stat: bool,

    #[arg(short = 'n', long = "no-index")]
    pub(crate) no_index: bool,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct QueryModeArgs {
    #[arg(short = 'x', long = "swap-fuzzy-exact")]
    pub(crate) swap_fuzzy: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Command {
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
    // Hidden commands are the shell-integration protocol: the shell templates
    // invoke these __* subcommands and apply the emitted PATH value themselves.
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
pub(crate) struct DiffArgs {
    #[arg(value_name = "SHELL")]
    pub(crate) shell: Option<String>,

    /// Show unchanged entries in addition to changes
    #[arg(long = "full")]
    pub(crate) full: bool,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct ApplyArgs {
    #[arg(value_name = "SHELL")]
    pub(crate) shell: Option<String>,

    /// Skip protected paths (apply minimal `PATH` without safety)
    #[arg(long = "no-protect")]
    pub(crate) no_protect: bool,
}

#[derive(ClapArgs, Debug, Default)]
pub(crate) struct UndoArgs {
    #[arg(value_name = "COUNT", default_value = "1")]
    pub(crate) count: usize,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct SaveProfileArgs {
    #[arg(value_name = "NAME", required = true)]
    pub(crate) name: String,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct LoadProfileArgs {
    #[arg(value_name = "NAME", required = true)]
    pub(crate) name: String,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct RemoveProfileArgs {
    #[arg(value_name = "NAME", required = true)]
    pub(crate) name: String,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenUndoArgs {
    #[arg(value_name = "COUNT", default_value = "1")]
    pub(crate) count: usize,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenRedoArgs {
    #[arg(value_name = "COUNT", default_value = "1")]
    pub(crate) count: usize,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenLoadArgs {
    #[arg(value_name = "NAME", required = true)]
    pub(crate) name: String,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct InitArgs {
    #[arg(value_name = "SHELL")]
    pub(crate) shell: String,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenMoveArgs {
    #[arg(value_name = "FROM")]
    pub(crate) from: usize,

    #[arg(value_name = "TO")]
    pub(crate) to: usize,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenSwapArgs {
    #[arg(value_name = "FIRST")]
    pub(crate) first: usize,

    #[arg(value_name = "SECOND")]
    pub(crate) second: usize,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenDeleteArgs {
    #[arg(value_name = "TARGET", required = true)]
    pub(crate) targets: Vec<String>,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenPreferArgs {
    #[arg(value_name = "ARGS", required = true)]
    pub(crate) tokens: Vec<String>,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenInitArgs {
    #[arg(value_name = "PID", required = true)]
    pub(crate) session_pid: u32,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenLoadSavedPathArgs {
    #[arg(value_name = "SHELL", required = true)]
    pub(crate) shell: String,
}

#[derive(ClapArgs, Debug)]
pub(crate) struct HiddenAddArgs {
    /// Paths to add to `PATH`
    #[arg(value_name = "PATH", required = true)]
    pub(crate) paths: Vec<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub(crate) enum ColorChoice {
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
