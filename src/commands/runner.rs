use std::env;
use std::io::{self, BufWriter, StdoutLock};
use std::path::PathBuf;

use crate::cli::args::Args;
use crate::config;

mod path_ops;
mod query;
mod session;

fn handle_path_result(
    result: Result<String, String>,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    match result {
        Ok(new_path) => {
            query::write_snapshot_safe(&new_path, args);
            query::output_path(out, &new_path)
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error: {e}");
            }
            2
        }
    }
}

fn get_current_exe_dir() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|exe_path| exe_path.parent().map(std::path::Path::to_path_buf))
}

#[must_use]
pub fn run(args: &Args) -> i32 {
    if let Err(e) = config::runtime::ensure_config_exists() {
        eprintln!("Error: {e}");
        return 2;
    }

    let config = config::runtime::load_config().unwrap_or_default();

    if let Some(ref shell) = args.init_shell {
        return session::handle_init(shell);
    }

    if let Some(apply_target) = &args.apply_target {
        return session::handle_apply(apply_target, args.no_protect);
    }

    if let Some(profile_name) = &args.save_profile {
        return session::handle_save_profile(profile_name);
    }

    if let Some(profile_name) = &args.load_profile {
        return session::handle_load_profile(profile_name);
    }

    if let Some(profile_name) = &args.remove_profile {
        return session::handle_remove_profile(profile_name);
    }

    if args.reset {
        return session::handle_reset();
    }

    if let Some(history_action) = &args.history_action {
        return match history_action {
            crate::cli::args::HistoryAction::Undo(count) => session::handle_undo(*count),
            crate::cli::args::HistoryAction::Redo(count) => session::handle_redo(*count),
        };
    }

    if args.diff {
        return session::handle_diff(args.diff_full);
    }

    let path_var = match &args.path_override {
        Some(p) => p.clone(),
        None => env::var("PATH").unwrap_or_default(),
    };

    let searcher = crate::path::searcher::PathSearcher::new(&path_var);
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    if args.clean {
        return path_ops::handle_clean(&searcher, args, &mut out);
    }

    if !args.delete_targets.is_empty() {
        return path_ops::handle_delete(&searcher, &args.delete_targets, args, &mut out);
    }

    if let Some(path_edit) = &args.path_edit {
        return path_ops::handle_move_or_swap(&searcher, path_edit, args, &mut out);
    }

    if let Some(ref target) = args.prefer_target {
        return path_ops::handle_prefer(&searcher, target, args, &mut out);
    }

    query::run_query(&searcher, args, &config, &mut out)
}

mod atty {
    use crate::platform;
    use std::os::unix::io::AsRawFd;

    pub fn is(stream: Stream) -> bool {
        let fd = match stream {
            Stream::Stdout => std::io::stdout().as_raw_fd(),
            Stream::Stdin => std::io::stdin().as_raw_fd(),
        };

        platform::is_tty(fd)
    }

    #[derive(Copy, Clone)]
    pub enum Stream {
        Stdout,
        Stdin,
    }
}
