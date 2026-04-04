use std::env;
use std::io::Write;

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
            if let Err(e) = history.write_snapshot(new_path)
                && !args.quiet
                && !args.silent
            {
                eprintln!("Warning: Failed to write snapshot: {e}");
            }
        }
        Err(e) => {
            if !args.quiet && !args.silent {
                eprintln!("Warning: Failed to acquire history: {e}");
            }
        }
    }
}

pub fn output_path<W: Write>(out: &mut W, new_path: &str) -> i32 {
    let original_path = env::var("PATH").unwrap_or_default();
    let guarded_path =
        PathGuard::default().ensure_protected_paths(&original_path, new_path.to_string());

    if let Err(err) = writeln!(out, "{guarded_path}") {
        eprintln!("Error: Failed to write PATH output: {err}");
        return 2;
    }
    if let Err(err) = out.flush() {
        eprintln!("Error: Failed to flush PATH output: {err}");
        return 2;
    }

    0
}

pub fn emit_line<W: Write>(out: &mut W, line: &str) -> i32 {
    if let Err(err) = writeln!(out, "{line}") {
        eprintln!("Error: Failed to write output: {err}");
        return 2;
    }
    if let Err(err) = out.flush() {
        eprintln!("Error: Failed to flush output: {err}");
        return 2;
    }

    0
}

pub fn should_use_color(args: &Args, stdout_is_tty: bool) -> bool {
    match args.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => stdout_is_tty,
    }
}

pub fn warn_if_loud(args: &Args, message: &str) {
    if !args.quiet && !args.silent {
        eprintln!("Warning: {message}");
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
