use std::path::PathBuf;

use crate::cli::{self, Args as AppArgs, HistoryAction, PathEdit};
use crate::commands;
use crate::config::shell_paths;
use crate::path::file::{apply_path_sections, expand_shell_vars};
use crate::path::guard::PathGuard;
use crate::path::resolve::resolve_path;
use crate::path::searcher::PathSearcher;
use crate::session::history::HistoryContext;
use crate::session::store;
use crate::shell::detect::Shell;

use super::spec::{
    HiddenAddArgs, HiddenDeleteArgs, HiddenInitArgs, HiddenLoadArgs, HiddenLoadSavedPathArgs,
    HiddenMoveArgs, HiddenPreferArgs, HiddenRedoArgs, HiddenSwapArgs, HiddenUndoArgs,
};

pub(super) fn run_hidden_move(opts: &HiddenMoveArgs) -> i32 {
    let args = AppArgs {
        path_edit: Some(PathEdit::Move {
            from: opts.from,
            to: opts.to,
        }),
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_hidden_swap(opts: &HiddenSwapArgs) -> i32 {
    let args = AppArgs {
        path_edit: Some(PathEdit::Swap {
            first: opts.first,
            second: opts.second,
        }),
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_hidden_clean() -> i32 {
    let args = AppArgs {
        clean: true,
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_hidden_delete(opts: HiddenDeleteArgs) -> i32 {
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

pub(super) fn run_hidden_prefer(opts: HiddenPreferArgs) -> i32 {
    run_prefer_tokens(opts.tokens)
}

pub(super) fn run_prefer_tokens(tokens: Vec<String>) -> i32 {
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

pub(super) fn run_hidden_reset() -> i32 {
    let args = AppArgs {
        reset: true,
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_hidden_undo(opts: &HiddenUndoArgs) -> i32 {
    let args = AppArgs {
        history_action: Some(HistoryAction::Undo(opts.count)),
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_hidden_redo(opts: &HiddenRedoArgs) -> i32 {
    let args = AppArgs {
        history_action: Some(HistoryAction::Redo(opts.count)),
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_hidden_load(opts: &HiddenLoadArgs) -> i32 {
    let session_pid = current_session_pid();

    match shell_paths::load_profile(&opts.name) {
        Ok(parsed) => {
            let current_path = std::env::var("PATH").unwrap_or_default();
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

            if let Ok(history) = HistoryContext::global(session_pid)
                && let Err(err) = history.write_snapshot(&expanded_path)
            {
                eprintln!("Warning: Failed to write profile snapshot: {err}");
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

pub(super) fn run_hidden_init(args: &HiddenInitArgs) -> i32 {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let session_pid = args.session_pid;

    match HistoryContext::global(session_pid) {
        Ok(history) => {
            if let Err(e) = history.reset_with_initial(&path_var) {
                eprintln!("Error: Failed to initialize session: {e}");
                return 2;
            }

            if let Err(err) = store::cleanup_old_sessions() {
                eprintln!("Warning: Failed to clean up old sessions: {err}");
            }

            0
        }
        Err(e) => {
            eprintln!("Error: Failed to prepare session history: {e}");
            2
        }
    }
}

pub(super) fn run_hidden_load_saved_path(args: &HiddenLoadSavedPathArgs) -> i32 {
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

pub(super) fn run_hidden_add(args: &HiddenAddArgs) -> i32 {
    let session_pid = current_session_pid();

    let paths = match cli::parse_add_arguments(args.paths.clone()) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {e}");
            return 2;
        }
    };

    let current_path = std::env::var("PATH").unwrap_or_default();
    let mut searcher = PathSearcher::new(&current_path);

    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    for path_str in paths {
        let resolved = match resolve_path(&path_str, &cwd) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("Warning: Could not resolve path '{path_str}': {e}");
                PathBuf::from(path_str)
            }
        };

        if searcher.contains(&resolved) {
            continue;
        }

        if let Err(e) = searcher.insert_at(&resolved, 1) {
            eprintln!("Warning: Could not add '{}': {}", resolved.display(), e);
        }
    }

    let new_path = searcher.to_path_string();

    if let Ok(history) = HistoryContext::global(session_pid)
        && let Err(err) = history.write_snapshot(&new_path)
    {
        eprintln!("Warning: Failed to write PATH snapshot: {err}");
    }

    let guarded_path = PathGuard::default().ensure_protected_paths(&current_path, new_path);
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
        name: "whin",
        command: "whi -n",
        description: "Hide PATH indices",
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

pub(super) fn run_shorthands() -> i32 {
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

fn current_session_pid() -> u32 {
    std::env::var("WHI_SESSION_PID")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(std::process::id)
}
