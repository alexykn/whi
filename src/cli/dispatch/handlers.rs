use clap::CommandFactory;

use crate::cli::{ApplyTarget, Args as AppArgs};
use crate::commands;
use crate::config::shell_paths;

use super::check_shell_integration;
use super::spec::{
    ApplyArgs, Cli, ColorChoice, DiffArgs, InitArgs, RemoveProfileArgs, SaveProfileArgs,
};

pub(super) fn run_query(opts: super::spec::QueryArgs) -> i32 {
    let args = AppArgs {
        names: opts.names,
        all: opts.listing.all,
        full: opts.listing.full,
        follow_symlinks: opts.listing.follow_symlinks,
        print0: opts.output.print0,
        quiet: opts.output.quiet,
        silent: opts.output.silent,
        one: opts.listing.one,
        show_nonexec: opts.listing.show_nonexec,
        path_override: opts.path_override,
        color: opts.color.unwrap_or(ColorChoice::Auto).into(),
        stat: opts.output.stat,
        no_index: opts.output.no_index,
        swap_fuzzy: opts.mode.swap_fuzzy,
        ..Default::default()
    };

    commands::run(&args)
}

pub(super) fn run_diff(opts: DiffArgs) -> i32 {
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

pub(super) fn run_apply(opts: ApplyArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        apply_target: Some(match opts.shell {
            Some(shell) => ApplyTarget::Shell(shell),
            None => ApplyTarget::CurrentShell,
        }),
        no_protect: opts.no_protect,
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_save_profile(opts: SaveProfileArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        save_profile: Some(opts.name),
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_remove_profile(opts: RemoveProfileArgs) -> i32 {
    if let Some(code) = check_shell_integration() {
        return code;
    }

    let args = AppArgs {
        remove_profile: Some(opts.name),
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_list_profiles() -> i32 {
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

pub(super) fn run_init(opts: InitArgs) -> i32 {
    let args = AppArgs {
        init_shell: Some(opts.shell),
        ..Default::default()
    };
    commands::run(&args)
}

pub(super) fn run_help() -> i32 {
    if let Err(err) = Cli::command().print_help() {
        eprintln!("Error: Failed to print help: {err}");
        return 2;
    }
    println!();
    0
}
