use std::env;
use std::fs;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

mod cli;
mod executor;
mod output;
mod path;
mod shell_integration;

use cli::{Args, ColorWhen};
use executor::{ExecutableCheck, SearchResult};
use output::OutputFormatter;
use path::PathSearcher;

fn main() {
    let args = match Args::parse() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(2);
        }
    };

    if args.silent {
        // Suppress all stderr
        let result = run(&args);
        process::exit(result);
    }

    match run(&args) {
        0 => process::exit(0),
        code => process::exit(code),
    }
}

#[allow(clippy::too_many_lines)]
fn run(args: &Args) -> i32 {
    // Handle init subcommand
    if let Some(ref shell) = args.init_shell {
        match shell_integration::generate_init_script(shell) {
            Ok(script) => {
                print!("{script}");
                return 0;
            }
            Err(e) => {
                eprintln!("Error: {e}");
                return 2;
            }
        }
    }

    let path_var = match &args.path_override {
        Some(p) => p.clone(),
        None => env::var("PATH").unwrap_or_default(),
    };

    let searcher = PathSearcher::new(&path_var);
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    // Handle --move operation
    if let Some((from, to)) = args.move_indices {
        match searcher.move_entry(from, to) {
            Ok(new_path) => {
                writeln!(out, "{new_path}").ok();
                out.flush().ok();
                return 0;
            }
            Err(e) => {
                if !args.silent {
                    eprintln!("Error: {e}");
                }
                return 2;
            }
        }
    }

    // Handle --swap operation
    if let Some((idx1, idx2)) = args.swap_indices {
        match searcher.swap_entries(idx1, idx2) {
            Ok(new_path) => {
                writeln!(out, "{new_path}").ok();
                out.flush().ok();
                return 0;
            }
            Err(e) => {
                if !args.silent {
                    eprintln!("Error: {e}");
                }
                return 2;
            }
        }
    }

    // Handle --prefer operation
    if let Some((ref name, target_idx)) = args.prefer_target {
        return handle_prefer(&searcher, name, target_idx, args, &mut out);
    }

    let names = get_names(args);

    // If no names provided, show all PATH entries
    if names.is_empty() {
        for (idx, dir) in searcher.dirs().iter().enumerate() {
            writeln!(out, "[{}] {}", idx + 1, dir.display()).ok();
        }
        out.flush().ok();
        return 0;
    }

    let mut all_found = true;
    let stderr = io::stderr();
    let mut err = BufWriter::new(stderr.lock());

    let use_color = should_use_color(args);
    let mut formatter = OutputFormatter::new(use_color, args.print0);

    for name in names {
        let results = search_name(&searcher, &name, args);

        if results.is_empty() {
            all_found = false;

            if !args.silent && !args.quiet {
                writeln!(err, "{name}: not found").ok();
            }
            continue;
        }

        // Output results
        for (i, result) in results.iter().enumerate() {
            let is_winner = i == 0;

            formatter
                .write_result(
                    &mut out,
                    result,
                    is_winner,
                    args.follow_symlinks,
                    args.index,
                )
                .ok();

            // By default, only show the winner (like `which`)
            // Show all with --all flag
            if !args.all || args.one {
                break;
            }
        }

        // If -f/--full, show full PATH listing after results
        if args.full {
            writeln!(out).ok();
            for (idx, dir) in searcher.dirs().iter().enumerate() {
                if args.index {
                    writeln!(out, "[{}] {}", idx + 1, dir.display()).ok();
                } else {
                    writeln!(out, "{}", dir.display()).ok();
                }
            }
        }
    }

    out.flush().ok();
    err.flush().ok();

    i32::from(!all_found)
}

fn get_names(args: &Args) -> Vec<String> {
    if !args.names.is_empty() {
        return args.names.clone();
    }

    // Only read from stdin if it's piped (not a TTY)
    if !atty::is(atty::Stream::Stdin) {
        let stdin = io::stdin();
        let mut names = Vec::new();
        for line in stdin.lock().lines().map_while(Result::ok) {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                names.push(trimmed.to_string());
            }
        }
        return names;
    }

    // No names and stdin is a TTY - return empty
    Vec::new()
}

fn search_name(searcher: &PathSearcher, name: &str, args: &Args) -> Vec<SearchResult> {
    // If name contains path separator, check it directly
    if name.contains('/') {
        let path = PathBuf::from(name);
        if let Some(result) = check_path(&path, args, 0) {
            return vec![result];
        }
        return vec![];
    }

    let mut results = Vec::new();

    for (idx, dir) in searcher.dirs().iter().enumerate() {
        let candidate = dir.join(name);
        if let Some(result) = check_path(&candidate, args, idx + 1) {
            results.push(result);
        }
    }

    results
}

fn check_path(path: &Path, args: &Args, path_index: usize) -> Option<SearchResult> {
    let checker = ExecutableCheck::new(path);

    if !checker.exists() {
        return None;
    }

    let is_executable = checker.is_executable();

    if !is_executable && !args.show_nonexec {
        return None;
    }

    let canonical_path = if args.follow_symlinks {
        fs::canonicalize(path).ok()
    } else {
        None
    };

    let metadata = if args.stat {
        checker.get_file_metadata()
    } else {
        None
    };

    Some(SearchResult {
        path: path.to_path_buf(),
        canonical_path,
        metadata,
        path_index,
    })
}

fn should_use_color(args: &Args) -> bool {
    match args.color {
        ColorWhen::Always => true,
        ColorWhen::Never => false,
        ColorWhen::Auto => atty::is(atty::Stream::Stdout),
    }
}

fn handle_prefer<W: Write>(
    searcher: &PathSearcher,
    name: &str,
    target_idx: usize,
    args: &Args,
    out: &mut W,
) -> i32 {
    // Search for all occurrences of the executable
    let results = search_name(searcher, name, args);

    if results.is_empty() {
        if !args.silent {
            eprintln!("Error: {name}: not found");
        }
        return 1;
    }

    // Find the current winner (first occurrence)
    let winner_idx = results[0].path_index;

    // Check if target_idx is in the results
    let target_result = results.iter().find(|r| r.path_index == target_idx);
    if target_result.is_none() {
        if !args.silent {
            eprintln!("Error: {name} not found at index {target_idx}");
        }
        return 2;
    }

    // Calculate the minimal move: move target to just before the winner
    let new_position = if target_idx > winner_idx {
        winner_idx
    } else {
        // Already before winner, no change needed
        if !args.silent {
            eprintln!("Error: {name} at index {target_idx} is already preferred over index {winner_idx}");
        }
        return 2;
    };

    // Perform the move
    match searcher.move_entry(target_idx, new_position) {
        Ok(new_path) => {
            writeln!(out, "{new_path}").ok();
            out.flush().ok();
            0
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error: {e}");
            }
            2
        }
    }
}

// TTY detection using isatty(3)
mod atty {
    use std::os::unix::io::AsRawFd;

    pub fn is(stream: Stream) -> bool {
        let fd = match stream {
            Stream::Stdout => std::io::stdout().as_raw_fd(),
            Stream::Stdin => std::io::stdin().as_raw_fd(),
        };

        // SAFETY: isatty is safe to call with any file descriptor
        unsafe { libc::isatty(fd) == 1 }
    }

    #[derive(Copy, Clone)]
    pub enum Stream {
        Stdout,
        Stdin,
    }
}
