use std::env;
use std::fs;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::cli::{Args, ColorWhen};
use crate::executor::{ExecutableCheck, SearchResult};
use crate::output::OutputFormatter;
use crate::path::PathSearcher;
use crate::path_resolver;
use crate::session_tracker;
use crate::shell_integration;
use crate::system;

#[allow(clippy::too_many_lines)]
pub fn run(args: &Args) -> i32 {
    // Handle init subcommand
    if let Some(ref shell) = args.init_shell {
        match shell_integration::generate_init_script(shell) {
            Ok(script) => {
                print!("{script}");
                return 0;
            }
            Err(err) => {
                eprintln!("Error: {err}");
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
                if let Ok(ppid) = system::get_parent_pid() {
                    if let Err(e) =
                        session_tracker::write_operation(ppid, "deleted", &removed_paths)
                    {
                        if !args.quiet && !args.silent {
                            eprintln!("Warning: Failed to log operation: {e}");
                        }
                    }
                }
            }
        }

        writeln!(out, "{new_path}").ok();
        out.flush().ok();
        return 0;
    }

    // Handle --delete operation
    if !args.delete_targets.is_empty() {
        return handle_delete(&searcher, &args.delete_targets, args, &mut out);
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
                    if let Ok(ppid) = system::get_parent_pid() {
                        if let Err(e) = session_tracker::write_operation(ppid, "moved", &[path]) {
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
                    if let Ok(ppid) = system::get_parent_pid() {
                        if let Err(e) =
                            session_tracker::write_operation(ppid, "swapped", &swapped_paths)
                        {
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
            Err(e) => {
                if !args.silent {
                    eprintln!("Error: {e}");
                }
                return 2;
            }
        }
    }

    // Handle --prefer operation
    if let Some(ref target) = args.prefer_target {
        return handle_prefer(&searcher, target, args, &mut out);
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
            if !args.no_index {
                writeln!(out, "{:>4} {}", format!("[{}]", idx + 1), dir.display()).ok();
            } else {
                writeln!(out, "{}", dir.display()).ok();
            }
        }
        out.flush().ok();
        return 0;
    }

    let mut all_found = true;
    if names.is_empty() {
        eprintln!("Usage: whi [OPTIONS] [NAME]...\n       whi <COMMAND>\n\nTry 'whi --help' for more information.");
        return 2;
    }

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
                    !args.no_index,
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

                if !args.no_index {
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

/// Get the directory containing the current whi executable
fn get_current_exe_dir() -> Option<PathBuf> {
    env::current_exe()
        .ok()
        .and_then(|exe_path| exe_path.parent().map(|p| p.to_path_buf()))
}

fn handle_prefer<W: Write>(
    searcher: &PathSearcher,
    target: &crate::cli::PreferTarget,
    args: &Args,
    out: &mut W,
) -> i32 {
    use crate::cli::PreferTarget;

    match target {
        PreferTarget::IndexBased { name, index } => {
            handle_prefer_index(searcher, name, *index, args, out)
        }
        PreferTarget::PathBased { name, path } => {
            handle_prefer_path(searcher, name, path, args, out)
        }
        PreferTarget::PathOnly { path } => handle_prefer_path_only(searcher, path, args, out),
    }
}

fn handle_prefer_index<W: Write>(
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
                if let Ok(ppid) = system::get_parent_pid() {
                    if let Err(e) = session_tracker::write_operation(ppid, "preferred", &[path]) {
                        if !args.quiet && !args.silent {
                            eprintln!("Warning: Failed to log operation: {e}");
                        }
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

fn handle_prefer_path<W: Write>(
    searcher: &PathSearcher,
    name: &str,
    path_str: &str,
    args: &Args,
    out: &mut W,
) -> i32 {
    use path_resolver::{looks_like_exact_path, resolve_path};

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Determine if this is an exact path or fuzzy pattern
    if looks_like_exact_path(path_str) {
        // Exact path - resolve it
        match resolve_path(path_str, &cwd) {
            Ok(resolved_path) => {
                handle_prefer_exact_path(searcher, name, &resolved_path, args, out)
            }
            Err(e) => {
                if !args.silent {
                    eprintln!("Error resolving path: {}", e);
                }
                2
            }
        }
    } else {
        // Fuzzy pattern
        handle_prefer_fuzzy(searcher, name, path_str, args, out)
    }
}

fn handle_prefer_exact_path<W: Write>(
    searcher: &PathSearcher,
    name: &str,
    path: &Path,
    args: &Args,
    out: &mut W,
) -> i32 {
    // Check if executable exists in the directory
    if !path.exists() {
        if !args.silent {
            eprintln!("Error: Directory does not exist: {}", path.display());
        }
        return 2;
    }

    if !searcher.has_executable(path, name) && !args.silent {
        eprintln!("Warning: {} not found in {}", name, path.display());
    }
    // Continue anyway - might be added later

    // Check if path already exists in PATH
    if let Some(idx) = searcher.find_path_index(path) {
        // Path already in PATH - use traditional index-based prefer
        return handle_prefer_index(searcher, name, idx, args, out);
    }

    // Path not in PATH - need to add it at the right position
    // First, find where the executable currently wins (if it exists)
    let results = search_name(searcher, name, args);

    let insert_position = if !results.is_empty() {
        // Executable exists - add new path just before the current winner
        results[0].path_index
    } else {
        // Executable doesn't exist anywhere - add at the beginning
        1
    };

    // Add the path at the calculated position
    match searcher.add_path_at_position(path, insert_position) {
        Ok(new_path) => {
            if !args.silent {
                eprintln!(
                    "Added {} to PATH at index {}",
                    path.display(),
                    insert_position
                );
            }

            // Log the addition
            if let Ok(ppid) = system::get_parent_pid() {
                if let Err(e) = session_tracker::write_operation(
                    ppid,
                    "preferred",
                    &[path.display().to_string()],
                ) {
                    if !args.quiet && !args.silent {
                        eprintln!("Warning: Failed to log operation: {e}");
                    }
                }
            }

            // Output the new PATH
            writeln!(out, "{}", new_path).ok();
            out.flush().ok();
            0
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error adding to PATH: {}", e);
            }
            2
        }
    }
}

fn handle_prefer_path_only<W: Write>(
    searcher: &PathSearcher,
    path_str: &str,
    args: &Args,
    out: &mut W,
) -> i32 {
    use path_resolver::{looks_like_exact_path, resolve_path};

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    // Resolve the path
    let resolved_path = if looks_like_exact_path(path_str) {
        match resolve_path(path_str, &cwd) {
            Ok(path) => path,
            Err(e) => {
                if !args.silent {
                    eprintln!("Error resolving path: {}", e);
                }
                return 2;
            }
        }
    } else {
        // Not a path-like string - treat as relative
        cwd.join(path_str)
    };

    // Check if path already exists in PATH
    if let Some(_idx) = searcher.find_path_index(&resolved_path) {
        // Already in PATH - do nothing (no duplicate)
        if !args.silent {
            eprintln!("{} is already in PATH", resolved_path.display());
        }
        // Return current PATH unchanged
        writeln!(out, "{}", searcher.to_path_string()).ok();
        out.flush().ok();
        return 0;
    }

    // Add to PATH at the beginning
    match searcher.add_path(&resolved_path) {
        Ok((new_path, idx)) => {
            if !args.silent {
                eprintln!("Added {} to PATH at index {}", resolved_path.display(), idx);
            }

            // Log the addition
            if let Ok(ppid) = system::get_parent_pid() {
                if let Err(e) = session_tracker::write_operation(
                    ppid,
                    "added",
                    &[resolved_path.display().to_string()],
                ) {
                    if !args.quiet && !args.silent {
                        eprintln!("Warning: Failed to log operation: {e}");
                    }
                }
            }

            writeln!(out, "{}", new_path).ok();
            out.flush().ok();
            0
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error adding to PATH: {}", e);
            }
            2
        }
    }
}

fn handle_prefer_fuzzy<W: Write>(
    searcher: &PathSearcher,
    name: &str,
    pattern: &str,
    args: &Args,
    out: &mut W,
) -> i32 {
    // Find matching paths
    let matches = searcher.find_fuzzy_indices(pattern, Some(name));

    if matches.is_empty() {
        if !args.silent {
            eprintln!(
                "Error: No PATH entries match pattern '{}' containing '{}'",
                pattern, name
            );
        }
        return 1;
    }

    if matches.len() > 1 {
        if !args.silent {
            eprintln!("Error: Multiple PATH entries match pattern '{}':", pattern);
            for (idx, path) in &matches {
                eprintln!("  [{}] {}", idx, path.display());
            }
            eprintln!("Please be more specific or use an index directly.");
        }
        return 2;
    }

    // Single match - use it
    let (index, _) = matches[0];
    handle_prefer_index(searcher, name, index, args, out)
}

fn handle_delete<W: Write>(
    searcher: &PathSearcher,
    targets: &[crate::cli::DeleteTarget],
    args: &Args,
    out: &mut W,
) -> i32 {
    use crate::cli::DeleteTarget;
    use crate::path_resolver::{looks_like_exact_path, resolve_path};

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut indices_to_delete = Vec::new();

    for target in targets {
        match target {
            DeleteTarget::Index(idx) => {
                indices_to_delete.push(*idx);
            }

            DeleteTarget::Path(path_str) => {
                if looks_like_exact_path(path_str) {
                    // Exact path - resolve it
                    match resolve_path(path_str, &cwd) {
                        Ok(resolved) => {
                            if let Some(idx) = searcher.find_path_index(&resolved) {
                                indices_to_delete.push(idx);
                            } else {
                                if !args.silent {
                                    eprintln!(
                                        "Error: Path not found in PATH: {}",
                                        resolved.display()
                                    );
                                }
                                return 1;
                            }
                        }
                        Err(e) => {
                            if !args.silent {
                                eprintln!("Error resolving path: {}", e);
                            }
                            return 2;
                        }
                    }
                } else {
                    // Fuzzy pattern - delete ALL matches
                    let matches = searcher.find_fuzzy_indices(path_str, None);

                    if matches.is_empty() {
                        if !args.silent {
                            eprintln!("Error: No PATH entries match pattern '{}'", path_str);
                        }
                        return 1;
                    }

                    // Add all matching indices (delete ALL matches)
                    for (idx, _) in &matches {
                        indices_to_delete.push(*idx);
                    }
                }
            }
        }
    }

    // Get the paths to be deleted before deletion (for logging)
    let dirs = searcher.dirs();

    // Filter out the directory containing the current whi executable (silently)
    if let Some(exe_dir) = get_current_exe_dir() {
        // Try to canonicalize for better matching
        let canonical_exe_dir = fs::canonicalize(&exe_dir).unwrap_or_else(|_| exe_dir.clone());

        indices_to_delete.retain(|&idx| {
            if idx > 0 && idx <= dirs.len() {
                let path = &dirs[idx - 1];
                // Compare both as-is and canonicalized paths
                let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());

                // Keep the index if it doesn't match the executable's directory
                path != &exe_dir
                    && path != &canonical_exe_dir
                    && canonical_path != exe_dir
                    && canonical_path != canonical_exe_dir
            } else {
                true
            }
        });
    }

    // Remove duplicates before displaying
    indices_to_delete.sort_unstable();
    indices_to_delete.dedup();

    // Show list of entries being deleted (for multi-delete operations)
    if !args.silent && indices_to_delete.len() > 1 {
        for &idx in &indices_to_delete {
            if idx > 0 && idx <= dirs.len() {
                eprintln!("{:>4} {}", format!("[{}]", idx), dirs[idx - 1].display());
            }
        }
    }

    let deleted_paths: Vec<String> = indices_to_delete
        .iter()
        .filter_map(|&idx| {
            if idx > 0 && idx <= dirs.len() {
                Some(dirs[idx - 1].display().to_string())
            } else {
                None
            }
        })
        .collect();

    let result = if indices_to_delete.len() == 1 {
        searcher.delete_entry(indices_to_delete[0])
    } else {
        searcher.delete_entries(&indices_to_delete)
    };

    match result {
        Ok(new_path) => {
            // Log deleted paths to session file
            if !deleted_paths.is_empty() {
                if let Ok(ppid) = system::get_parent_pid() {
                    if let Err(e) =
                        session_tracker::write_operation(ppid, "deleted", &deleted_paths)
                    {
                        if !args.quiet && !args.silent {
                            eprintln!("Warning: Failed to log operation: {e}");
                        }
                    }
                }
            }

            writeln!(out, "{}", new_path).ok();
            out.flush().ok();
            0
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error: {}", e);
            }
            2
        }
    }
}

fn handle_save(shell_opt: &Option<String>) -> i32 {
    use crate::config_manager::save_path;
    use crate::session_tracker::{cleanup_old_sessions, clear_session};
    use crate::shell_detect::{detect_current_shell, Shell};

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
                let shell = match shell_str.parse::<Shell>() {
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
        if let Ok(ppid) = system::get_parent_pid() {
            if let Err(e) = clear_session(ppid) {
                eprintln!("Warning: Failed to clear session log: {}", e);
            }
        }

        match cleanup_old_sessions() {
            Ok(count) if count > 0 => {
                eprintln!("Cleaned up {} old session file(s)", count);
            }
            Err(e) => {
                eprintln!("Warning: Failed to cleanup old sessions: {}", e);
            }
            _ => {}
        }
    }

    result
}

fn handle_diff(shell_opt: &Option<String>, full: bool) -> i32 {
    use crate::config_manager::load_saved_path;
    use crate::path_diff::{compute_diff, format_diff};
    use crate::session_tracker::read_session_paths;
    use crate::shell_detect::{detect_current_shell, Shell};

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
        Some(shell_str) => match shell_str.parse::<Shell>() {
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
    let (affected_paths, deleted_paths) = system::get_parent_pid()
        .ok()
        .and_then(|ppid| read_session_paths(ppid).ok())
        .unwrap_or_else(|| (std::collections::HashSet::new(), Vec::new()));

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

        crate::system::is_tty(fd)
    }

    #[derive(Copy, Clone)]
    pub enum Stream {
        Stdout,
        Stdin,
    }
}
