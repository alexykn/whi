use std::env;
use std::fs;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

mod cli;
mod config_manager;
mod executor;
mod output;
mod path;
mod path_diff;
mod session_tracker;
mod shell_detect;
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

    // Handle save subcommand
    if let Some(shell_opt) = &args.save_shell {
        return handle_save(shell_opt);
    }

    // Handle diff subcommand
    if let Some(shell_opt) = &args.diff_shell {
        return handle_diff(shell_opt, args.diff_full);
    }

    let path_var = match &args.path_override {
        Some(p) => p.clone(),
        None => env::var("PATH").unwrap_or_default(),
    };

    let searcher = PathSearcher::new(&path_var);
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    // Handle --clean operation
    if args.clean {
        let (new_path, removed_indices) = searcher.clean_duplicates();

        // Log deleted duplicates to session file
        if !removed_indices.is_empty() {
            // Get the actual paths that were removed
            let dirs = searcher.dirs();
            let removed_paths: Vec<String> = removed_indices
                .iter()
                .filter_map(|&idx| {
                    if idx > 0 && idx <= dirs.len() {
                        Some(dirs[idx - 1].display().to_string())
                    } else {
                        None
                    }
                })
                .collect();

            if !removed_paths.is_empty() {
                let ppid = unsafe { libc::getppid() as u32 };
                if let Err(e) = session_tracker::write_operation(ppid, "deleted", &removed_paths) {
                    if !args.quiet && !args.silent {
                        eprintln!("Warning: Failed to log operation: {e}");
                    }
                }
            }
        }

        writeln!(out, "{new_path}").ok();
        out.flush().ok();
        return 0;
    }

    // Handle --delete operation
    if !args.delete_indices.is_empty() {
        // Get the paths to be deleted before deletion
        let dirs = searcher.dirs();
        let deleted_paths: Vec<String> = args
            .delete_indices
            .iter()
            .filter_map(|&idx| {
                if idx > 0 && idx <= dirs.len() {
                    Some(dirs[idx - 1].display().to_string())
                } else {
                    None
                }
            })
            .collect();

        let result = if args.delete_indices.len() == 1 {
            searcher.delete_entry(args.delete_indices[0])
        } else {
            searcher.delete_entries(&args.delete_indices)
        };

        match result {
            Ok(new_path) => {
                // Log deleted paths to session file
                if !deleted_paths.is_empty() {
                    let ppid = unsafe { libc::getppid() as u32 };
                    if let Err(e) =
                        session_tracker::write_operation(ppid, "deleted", &deleted_paths)
                    {
                        if !args.quiet && !args.silent {
                            eprintln!("Warning: Failed to log operation: {e}");
                        }
                    }
                }

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

    // Handle --move operation
    if let Some((from, to)) = args.move_indices {
        // Get the path being moved
        let dirs = searcher.dirs();
        let moved_path = if from > 0 && from <= dirs.len() {
            Some(dirs[from - 1].display().to_string())
        } else {
            None
        };

        match searcher.move_entry(from, to) {
            Ok(new_path) => {
                // Log moved path to session file
                if let Some(path) = moved_path {
                    let ppid = unsafe { libc::getppid() as u32 };
                    if let Err(e) = session_tracker::write_operation(ppid, "moved", &[path]) {
                        if !args.quiet && !args.silent {
                            eprintln!("Warning: Failed to log operation: {e}");
                        }
                    }
                }

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
        // Get the paths being swapped
        let dirs = searcher.dirs();
        let mut swapped_paths = Vec::new();

        if idx1 > 0 && idx1 <= dirs.len() {
            swapped_paths.push(dirs[idx1 - 1].display().to_string());
        }
        if idx2 > 0 && idx2 <= dirs.len() {
            swapped_paths.push(dirs[idx2 - 1].display().to_string());
        }

        match searcher.swap_entries(idx1, idx2) {
            Ok(new_path) => {
                // Log swapped paths to session file
                if !swapped_paths.is_empty() {
                    let ppid = unsafe { libc::getppid() as u32 };
                    if let Err(e) =
                        session_tracker::write_operation(ppid, "swapped", &swapped_paths)
                    {
                        if !args.quiet && !args.silent {
                            eprintln!("Warning: Failed to log operation: {e}");
                        }
                    }
                }

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
        let num_dirs = searcher.dirs().len();
        if num_dirs > 999 {
            if !args.silent {
                eprintln!("Error: PATH has {num_dirs} entries (max 999 supported)");
            }
            return 3;
        }

        for (idx, dir) in searcher.dirs().iter().enumerate() {
            if args.index {
                writeln!(out, "{:>4} {}", format!("[{}]", idx + 1), dir.display()).ok();
            } else {
                writeln!(out, "{}", dir.display()).ok();
            }
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

        // Check max index
        let max_index = results.iter().map(|r| r.path_index).max().unwrap_or(0);
        if max_index > 999 {
            if !args.silent {
                eprintln!("Error: PATH index {max_index} exceeds max 999");
            }
            return 3;
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
                    3, // Always use 3-digit width
                )
                .ok();

            // By default, only show the winner (like `which`)
            // Show all with --all flag or --full flag (full implies all)
            if (!args.all && !args.full) || args.one {
                break;
            }
        }

        // If -f/--full, show full PATH listing after results
        if args.full {
            writeln!(out).ok();

            // Collect all path indices that contain matches
            let match_indices: std::collections::HashSet<usize> =
                results.iter().map(|r| r.path_index).collect();

            for (idx, dir) in searcher.dirs().iter().enumerate() {
                let path_index = idx + 1;
                let has_match = match_indices.contains(&path_index);

                if args.index {
                    write!(out, "{:>4} ", format!("[{}]", path_index)).ok();
                }

                if use_color && has_match {
                    // Use yellow/dim color for directories containing matches
                    writeln!(out, "\x1b[33m{}\x1b[0m", dir.display()).ok();
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
            eprintln!(
                "Error: {name} at index {target_idx} is already preferred over index {winner_idx}"
            );
        }
        return 2;
    };

    // Get the path being moved (before the move)
    let dirs = searcher.dirs();
    let preferred_path = if target_idx > 0 && target_idx <= dirs.len() {
        Some(dirs[target_idx - 1].display().to_string())
    } else {
        None
    };

    // Perform the move
    match searcher.move_entry(target_idx, new_position) {
        Ok(new_path) => {
            // Log preferred path to session file
            if let Some(path) = preferred_path {
                let ppid = unsafe { libc::getppid() as u32 };
                if let Err(e) = session_tracker::write_operation(ppid, "preferred", &[path]) {
                    if !args.quiet && !args.silent {
                        eprintln!("Warning: Failed to log operation: {e}");
                    }
                }
            }

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

fn handle_save(shell_opt: &Option<String>) -> i32 {
    use config_manager::save_path;
    use session_tracker::{cleanup_old_sessions, clear_session};
    use shell_detect::{detect_current_shell, Shell};

    let path_var = env::var("PATH").unwrap_or_default();

    let result = match shell_opt {
        None => {
            // Auto-detect current shell
            let shell = match detect_current_shell() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return 2;
                }
            };

            if let Err(e) = save_path(&shell, &path_var) {
                eprintln!("Error: {e}");
                return 2;
            }

            let num_entries = path_var.split(':').filter(|s| !s.is_empty()).count();
            println!("Saved PATH to {} ({} entries)", shell.as_str(), num_entries);
            0
        }
        Some(shell_str) => {
            if shell_str.to_lowercase() == "all" {
                // Save to all three shells
                let shells = [Shell::Bash, Shell::Zsh, Shell::Fish];
                let mut all_ok = true;

                for shell in &shells {
                    if let Err(e) = save_path(shell, &path_var) {
                        eprintln!("Error saving to {}: {e}", shell.as_str());
                        all_ok = false;
                    } else {
                        let num_entries = path_var.split(':').filter(|s| !s.is_empty()).count();
                        println!("Saved PATH to {} ({} entries)", shell.as_str(), num_entries);
                    }
                }

                if all_ok {
                    0
                } else {
                    2
                }
            } else {
                // Save to specific shell
                let shell = match Shell::from_str(shell_str) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        return 2;
                    }
                };

                if let Err(e) = save_path(&shell, &path_var) {
                    eprintln!("Error: {e}");
                    return 2;
                }

                let num_entries = path_var.split(':').filter(|s| !s.is_empty()).count();
                println!("Saved PATH to {} ({} entries)", shell.as_str(), num_entries);
                0
            }
        }
    };

    // After successful save, clear session log and cleanup old sessions
    if result == 0 {
        let ppid = unsafe { libc::getppid() as u32 };
        let _ = clear_session(ppid); // Ignore errors
        let _ = cleanup_old_sessions(); // Ignore errors
    }

    result
}

fn handle_diff(shell_opt: &Option<String>, full: bool) -> i32 {
    use config_manager::load_saved_path;
    use path_diff::{compute_diff, format_diff};
    use session_tracker::read_session_paths;
    use shell_detect::{detect_current_shell, Shell};

    let current_path = env::var("PATH").unwrap_or_default();
    let use_color = atty::is(atty::Stream::Stdout);

    let shell = match shell_opt {
        None => {
            // Auto-detect current shell
            match detect_current_shell() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return 2;
                }
            }
        }
        Some(shell_str) => match Shell::from_str(shell_str) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error: {e}");
                return 2;
            }
        },
    };

    let saved_path = match load_saved_path(&shell) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };

    // Get PPID (parent shell PID) to read affected and deleted paths
    let ppid = unsafe { libc::getppid() as u32 };
    let (affected_paths, deleted_paths) =
        read_session_paths(ppid).unwrap_or_else(|_| (std::collections::HashSet::new(), Vec::new()));

    let diff = compute_diff(
        &current_path,
        &saved_path,
        &affected_paths,
        &deleted_paths,
        full,
    );
    let formatted = format_diff(&diff, use_color);

    println!("{formatted}");

    0
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
