use std::process;

use clap::Parser;

use crate::config::{protected_paths, runtime};

mod handlers;
mod internal;
mod spec;

pub fn run() -> i32 {
    let cli_result = spec::Cli::try_parse();

    let spec::Cli { query, command } = match cli_result {
        Ok(cli) => cli,
        Err(err) => {
            let err_msg = err.to_string();
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

            if rewritten != err_msg {
                eprint!("{rewritten}");
                process::exit(2);
            }

            err.exit();
        }
    };

    if let Err(e) = runtime::ensure_config_exists() {
        eprintln!("Error: {e}");
        process::exit(2);
    }

    if let Err(e) = protected_paths::migrate_from_config_toml() {
        eprintln!("Warning: Failed to migrate protected paths from config.toml: {e}");
        eprintln!("Your configuration may not have been fully migrated.");
        eprintln!("Please check ~/.whi/protected_paths and ~/.whi/config.toml");
    }

    if let Err(e) = protected_paths::ensure_protected_paths_exists() {
        eprintln!("Warning: Failed to create protected_paths file: {e}");
    }

    match command {
        Some(spec::Command::Diff(diff)) => handlers::run_diff(diff),
        Some(spec::Command::Apply(apply)) => handlers::run_apply(apply),
        Some(spec::Command::Help) => handlers::run_help(),
        // Public PATH-manipulation commands are intentionally shell-facing only.
        // The shell integration wrappers translate them into the hidden __* protocol,
        // which is what actually mutates PATH and returns the new value to export.
        Some(
            spec::Command::Prefer
            | spec::Command::Move
            | spec::Command::Switch
            | spec::Command::Clean
            | spec::Command::Delete
            | spec::Command::Reset
            | spec::Command::Undo(_)
            | spec::Command::Redo(_)
            | spec::Command::Load(_)
            | spec::Command::Add,
        ) => check_shell_integration().unwrap_or(0),
        Some(spec::Command::Save(save)) => handlers::run_save_profile(save),
        Some(spec::Command::List) => handlers::run_list_profiles(),
        Some(spec::Command::RemoveProfile(remove)) => handlers::run_remove_profile(remove),
        Some(spec::Command::Init(init)) => handlers::run_init(init),
        Some(spec::Command::HiddenMove(move_args)) => internal::run_hidden_move(&move_args),
        Some(spec::Command::HiddenSwap(swap_args)) => internal::run_hidden_swap(&swap_args),
        Some(spec::Command::HiddenClean) => internal::run_hidden_clean(),
        Some(spec::Command::HiddenDelete(delete_args)) => internal::run_hidden_delete(delete_args),
        Some(spec::Command::HiddenPrefer(prefer_args)) => internal::run_hidden_prefer(prefer_args),
        Some(spec::Command::HiddenReset) => internal::run_hidden_reset(),
        Some(spec::Command::HiddenUndo(undo_args)) => internal::run_hidden_undo(&undo_args),
        Some(spec::Command::HiddenRedo(redo_args)) => internal::run_hidden_redo(&redo_args),
        Some(spec::Command::HiddenLoad(load_args)) => internal::run_hidden_load(&load_args),
        Some(spec::Command::HiddenInit(args)) => internal::run_hidden_init(&args),
        Some(spec::Command::HiddenLoadSavedPath(args)) => {
            internal::run_hidden_load_saved_path(&args)
        }
        Some(spec::Command::HiddenAdd(add_args)) => internal::run_hidden_add(&add_args),
        Some(spec::Command::Shorthands) => internal::run_shorthands(),
        None => handlers::run_query(query),
    }
}

fn check_shell_integration() -> Option<i32> {
    if std::env::var("WHI_SHELL_INITIALIZED").is_err() {
        eprintln!(
            "Shell integration not detected.\n\nRun one of these commands:\n  bash (current shell):    eval \"$(whi init bash)\"\n  bash (persistent):       add that line to the END of ~/.bashrc\n  zsh (current shell):     eval \"$(whi init zsh)\"\n  zsh (persistent):        add that line to the END of ~/.zshrc\n  fish (current shell):    whi init fish | source\n  fish (persistent):       add that line to the END of ~/.config/fish/config.fish\n"
        );
        return Some(2);
    }
    None
}
