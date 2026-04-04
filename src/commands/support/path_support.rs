use std::env;
use std::io::{BufWriter, StdoutLock, Write};

use crate::cli::args::{Args, ColorWhen};
use crate::path::guard::PathGuard;
use crate::session::history::HistoryContext;

pub fn history_for_current_scope() -> Result<HistoryContext, String> {
    let pid = get_session_pid().map_err(|e| e.to_string())?;
    HistoryContext::global(pid)
}

pub fn write_snapshot_safe(new_path: &str, args: &Args) {
    match history_for_current_scope() {
        Ok(history) => {
            if let Err(e) = history.write_snapshot(new_path) {
                if !args.quiet && !args.silent {
                    eprintln!("Warning: Failed to write snapshot: {e}");
                }
            }
        }
        Err(e) => {
            if !args.quiet && !args.silent {
                eprintln!("Warning: Failed to acquire history: {e}");
            }
        }
    }
}

pub fn output_path(out: &mut BufWriter<StdoutLock>, new_path: &str) -> i32 {
    let original_path = env::var("PATH").unwrap_or_default();
    let guarded_path =
        PathGuard::default().ensure_protected_paths(&original_path, new_path.to_string());

    writeln!(out, "{guarded_path}").ok();
    out.flush().ok();
    0
}

pub fn should_use_color(args: &Args) -> bool {
    match args.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => crate::platform::is_tty(1),
    }
}

fn get_session_pid() -> Result<u32, std::io::Error> {
    if let Ok(pid_str) = env::var("WHI_SESSION_PID") {
        pid_str.parse::<u32>().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid WHI_SESSION_PID value",
            )
        })
    } else {
        crate::platform::get_parent_pid()
    }
}
