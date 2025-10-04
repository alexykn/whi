use std::env;
use std::fs;
use std::io::{self, BufRead, BufWriter, StdoutLock, Write};
use std::path::{Path, PathBuf};

use crate::cli::{Args, ColorWhen};
use crate::executor::{ExecutableCheck, SearchResult};
use crate::history::{HistoryContext, HistoryScope};
use crate::output::OutputFormatter;
use crate::path::PathSearcher;
use crate::path_guard::PathGuard;
use crate::path_resolver;
use crate::shell_integration;
use crate::system;
use crate::venv_manager;

/// Get the session `PID` - either from `WHI_SESSION_PID` env var or fall back to parent `PID`
fn get_session_pid() -> Result<u32, std::io::Error> {
    if let Ok(pid_str) = env::var("WHI_SESSION_PID") {
        pid_str.parse::<u32>().map_err(|_| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid WHI_SESSION_PID value",
            )
        })
    } else {
        system::get_parent_pid()
    }
}

/// Write `PATH` snapshot to session tracker, with error handling
fn write_snapshot_safe(new_path: &str, args: &Args) {
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

fn history_for_current_scope() -> Result<HistoryContext, String> {
    let pid = get_session_pid().map_err(|e| e.to_string())?;

    if venv_manager::is_in_venv() {
        if let Ok(dir) = env::var("WHI_VENV_DIR") {
            if !dir.is_empty() {
                let path = PathBuf::from(dir);
                return HistoryContext::venv(pid, path.as_path());
            }
        }
    }

    HistoryContext::global(pid)
}

/// Output new `PATH` and flush, returning success code
fn output_path(out: &mut BufWriter<StdoutLock>, new_path: &str) -> i32 {
    // Apply path guard to preserve critical binaries (whi, zoxide)
    let original_path = env::var("PATH").unwrap_or_default();
    let guarded_path =
        PathGuard::default().ensure_protected_paths(&original_path, new_path.to_string());

    writeln!(out, "{guarded_path}").ok();
    out.flush().ok();
    0
}

/// Handle Result from `PATH` operation: write snapshot on success, print error on failure
fn handle_path_result(
    result: Result<String, String>,
    args: &Args,
    out: &mut BufWriter<StdoutLock>,
) -> i32 {
    match result {
        Ok(new_path) => {
            write_snapshot_safe(&new_path, args);
            output_path(out, &new_path)
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error: {e}");
            }
            2
        }
    }
}

#[allow(clippy::too_many_lines)]
#[must_use]
pub fn run(args: &Args) -> i32 {
    if let Err(e) = crate::config::ensure_config_exists() {
        eprintln!("Error: {e}");
        return 2;
    }

    // Load config for fuzzy search settings
    let config = crate::config::load_config().unwrap_or_default();

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

    // Handle apply subcommand (renamed from save)
    if let Some(shell_opt) = &args.apply_shell {
        return handle_apply(shell_opt.as_ref(), args.no_protect, args.apply_force);
    }

    // Handle save profile subcommand
    if let Some(profile_name) = &args.save_profile {
        return handle_save_profile(profile_name);
    }

    // Handle load profile subcommand
    if let Some(profile_name) = &args.load_profile {
        return handle_load_profile(profile_name);
    }

    // Handle remove profile subcommand
    if let Some(profile_name) = &args.remove_profile {
        return handle_remove_profile(profile_name);
    }

    // Handle reset subcommand
    if args.reset {
        return handle_reset();
    }

    // Handle undo subcommand
    if let Some(count) = args.undo_count {
        return handle_undo(count);
    }

    // Handle redo subcommand
    if let Some(count) = args.redo_count {
        return handle_redo(count);
    }

    // Handle diff subcommand
    if args.diff {
        return handle_diff(args.diff_full);
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
        let (new_path, _removed_indices) = searcher.clean_duplicates();
        write_snapshot_safe(&new_path, args);
        return output_path(&mut out, &new_path);
    }

    // Handle --delete operation
    if !args.delete_targets.is_empty() {
        return handle_delete(&searcher, &args.delete_targets, args, &mut out);
    }

    // Handle --move operation
    if let Some((from, to)) = args.move_indices {
        return handle_path_result(searcher.move_entry(from, to), args, &mut out);
    }

    // Handle --swap operation
    if let Some((idx1, idx2)) = args.swap_indices {
        return handle_path_result(searcher.swap_entries(idx1, idx2), args, &mut out);
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
            if args.no_index {
                writeln!(out, "{}", dir.display()).ok();
            } else {
                writeln!(out, "{:>4} {}", format!("[{}]", idx + 1), dir.display()).ok();
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
        // Determine fuzzy mode: config XOR swap flag
        let use_fuzzy = config.search.executable_search_fuzzy ^ args.swap_fuzzy;

        let results = if !name.contains('/') && use_fuzzy {
            // Fuzzy search enabled: search directly with fuzzy, no exact check
            search_name_fuzzy(&searcher, &name, args)
        } else {
            // Fuzzy disabled or path query: exact search only
            search_name(&searcher, &name, args)
        };

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

        // When fuzzy enabled: group by PATH index, then sort by name within each index
        // PATH order is ALWAYS respected - we never override it
        if !name.contains('/') && use_fuzzy {
            use std::collections::{BTreeMap, HashSet};

            // Group by PATH index (BTreeMap keeps them sorted)
            let mut by_index: BTreeMap<usize, Vec<&SearchResult>> = BTreeMap::new();
            for result in &results {
                by_index.entry(result.path_index).or_default().push(result);
            }

            // For each index, sort by executable name
            for index_results in by_index.values_mut() {
                index_results.sort_by(|a, b| {
                    let name_a = a.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let name_b = b.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    name_a.cmp(name_b)
                });
            }

            // Track which executable names we've seen (to determine winners)
            let mut seen_names: HashSet<String> = HashSet::new();

            // Output in PATH index order (BTreeMap keeps keys sorted)
            for index_results in by_index.into_values() {
                for result in &index_results {
                    let file_name = result
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    // Winner = first occurrence of this executable name
                    let is_winner = seen_names.insert(file_name.to_string());

                    // Without -a/-f, show ONLY winners
                    if !args.all && !args.full && !is_winner {
                        continue; // Skip non-winners
                    }

                    formatter
                        .write_result(
                            &mut out,
                            result,
                            is_winner,
                            args.follow_symlinks,
                            !args.no_index,
                            3,
                        )
                        .ok();
                }
            }
        } else {
            // Exact search: show results as before
            for (i, result) in results.iter().enumerate() {
                let is_winner = i == 0;

                formatter
                    .write_result(
                        &mut out,
                        result,
                        is_winner,
                        args.follow_symlinks,
                        !args.no_index,
                        3,
                    )
                    .ok();

                // By default, only show the winner
                if (!args.all && !args.full) || args.one {
                    break;
                }
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
    let search_all = args.all || args.full;

    for (idx, dir) in searcher.dirs().iter().enumerate() {
        let candidate = dir.join(name);
        if let Some(result) = check_path(&candidate, args, idx + 1) {
            results.push(result);

            // Stop after first match if not searching for all (like `which`)
            if !search_all {
                break;
            }
        }
    }

    results
}

/// Helper to check a directory entry with cached metadata
fn check_dir_entry(entry: &fs::DirEntry, args: &Args, path_index: usize) -> Option<SearchResult> {
    let path = entry.path();

    // Use fs::metadata (follows symlinks) instead of entry.metadata() (doesn't follow)
    // This ensures symlinked executables like /opt/homebrew/bin/zoxide are found
    let metadata = fs::metadata(&path).ok()?;

    // Only consider files (not directories)
    if !metadata.is_file() && !args.show_nonexec {
        return None;
    }

    let checker = ExecutableCheck::with_metadata(&path, metadata.clone());

    let is_executable = checker.is_executable();

    if !is_executable && !args.show_nonexec {
        return None;
    }

    let canonical_path = if args.follow_symlinks {
        fs::canonicalize(&path).ok()
    } else {
        None
    };

    let file_metadata = if args.stat {
        checker.get_file_metadata()
    } else {
        None
    };

    Some(SearchResult {
        path,
        canonical_path,
        metadata: file_metadata,
        path_index,
    })
}

/// Fuzzy search for executable names
fn search_name_fuzzy(searcher: &PathSearcher, query: &str, args: &Args) -> Vec<SearchResult> {
    use crate::path_resolver::FuzzyMatcher;
    use std::ffi::OsStr;

    let matcher = FuzzyMatcher::new(query);
    let mut results = Vec::new();

    // Always collect ALL fuzzy matches - the display logic decides what to show
    for (idx, dir) in searcher.dirs().iter().enumerate() {
        // Read directory entries
        let Ok(entries) = fs::read_dir(dir) else {
            continue; // Skip directories we can't read
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Check if filename matches fuzzy pattern
            let Some(filename) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };

            if matcher.matches(&PathBuf::from(filename)) {
                if let Some(result) = check_dir_entry(&entry, args, idx + 1) {
                    results.push(result);
                }
            }
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
        .and_then(|exe_path| exe_path.parent().map(std::path::Path::to_path_buf))
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
    // Need to search ALL occurrences for prefer logic to work
    let mut search_args = args.clone();
    search_args.all = true;
    let results = search_name(searcher, name, &search_args);

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

    match searcher.move_entry(target_idx, new_position) {
        Ok(new_path) => {
            write_snapshot_safe(&new_path, args);

            // Apply path guard to preserve critical binaries (whi, zoxide)
            let original_path = env::var("PATH").unwrap_or_default();
            let guarded_path =
                PathGuard::default().ensure_protected_paths(&original_path, new_path);

            writeln!(out, "{guarded_path}").ok();
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
                    eprintln!("Error resolving path: {e}");
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

    // Check if path already exists in PATH
    if let Some(idx) = searcher.find_path_index(path) {
        // Path already in PATH - use traditional index-based prefer
        return handle_prefer_index(searcher, name, idx, args, out);
    }

    // Path not in PATH yet - verify executable exists before adding
    if !searcher.has_executable(path, name) {
        if !args.silent {
            eprintln!("Error: {} not found in {}", name, path.display());
        }
        return 2;
    }

    // Path not in PATH - need to add it at the right position
    // First, find where the executable currently wins (if it exists)
    let results = search_name(searcher, name, args);

    let insert_position = if results.is_empty() {
        // Executable doesn't exist anywhere - add at the beginning
        1
    } else {
        // Executable exists - add new path just before the current winner
        results[0].path_index
    };

    match searcher.add_path_at_position(path, insert_position) {
        Ok(new_path) => {
            if !args.silent {
                eprintln!(
                    "Added {} to PATH at index {}",
                    path.display(),
                    insert_position
                );
            }

            write_snapshot_safe(&new_path, args);

            // Apply path guard to preserve critical binaries (whi, zoxide)
            let original_path = env::var("PATH").unwrap_or_default();
            let guarded_path =
                PathGuard::default().ensure_protected_paths(&original_path, new_path);

            writeln!(out, "{guarded_path}").ok();
            out.flush().ok();
            0
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error adding to PATH: {e}");
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
                    eprintln!("Error resolving path: {e}");
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

    match searcher.add_path(&resolved_path) {
        Ok((new_path, idx)) => {
            if !args.silent {
                eprintln!("Added {} to PATH at index {}", resolved_path.display(), idx);
            }

            write_snapshot_safe(&new_path, args);

            // Apply path guard to preserve critical binaries (whi, zoxide)
            let original_path = env::var("PATH").unwrap_or_default();
            let guarded_path =
                PathGuard::default().ensure_protected_paths(&original_path, new_path);

            writeln!(out, "{guarded_path}").ok();
            out.flush().ok();
            0
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error adding to PATH: {e}");
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
            eprintln!("Error: No PATH entries match pattern '{pattern}' containing '{name}'");
        }
        return 1;
    }

    if matches.len() > 1 {
        if !args.silent {
            eprintln!("Error: Multiple PATH entries match pattern '{pattern}':");
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
                                eprintln!("Error resolving path: {e}");
                            }
                            return 2;
                        }
                    }
                } else {
                    // Fuzzy pattern - delete ALL matches
                    let matches = searcher.find_fuzzy_indices(path_str, None);

                    if matches.is_empty() {
                        if !args.silent {
                            eprintln!("Error: No PATH entries match pattern '{path_str}'");
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

    // Show deleted paths in diff format (always, even for single deletions)
    if !args.silent && !indices_to_delete.is_empty() {
        let use_color = should_use_color(args);

        let (red, reset) = if use_color {
            ("\x1b[31m", "\x1b[0m")
        } else {
            ("", "")
        };

        for &idx in &indices_to_delete {
            if idx > 0 && idx <= dirs.len() {
                eprintln!("{red}- {}{reset}", dirs[idx - 1].display());
            }
        }
    }

    let result = if indices_to_delete.len() == 1 {
        searcher.delete_entry(indices_to_delete[0])
    } else {
        searcher.delete_entries(&indices_to_delete)
    };

    match result {
        Ok(new_path) => {
            write_snapshot_safe(&new_path, args);

            // Apply path guard to preserve critical binaries (whi, zoxide)
            let original_path = env::var("PATH").unwrap_or_default();
            let guarded_path =
                PathGuard::default().ensure_protected_paths(&original_path, new_path);

            writeln!(out, "{guarded_path}").ok();
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

#[allow(clippy::too_many_lines)]
fn handle_apply(shell_opt: Option<&String>, no_protect: bool, force: bool) -> i32 {
    use crate::config::load_config;
    use crate::config_manager::save_path;
    use crate::session_tracker::cleanup_old_sessions;
    use crate::shell_detect::{detect_current_shell, Shell};
    use std::collections::HashSet;

    if venv_manager::is_in_venv() && !force {
        eprintln!("Error: Refusing to run 'whi apply' inside an active PATH environment. Exit the venv or re-run with '--force' (optionally with '--no-protect').");
        return 2;
    }

    let mut path_var = env::var("PATH").unwrap_or_default();

    // Apply protected paths unless --no-protect is set
    // Protection is silent - just ensures configured paths are present
    if !no_protect {
        if let Ok(config) = load_config() {
            // Normalize paths by removing trailing slashes for comparison
            let current_paths: HashSet<String> = path_var
                .split(':')
                .filter(|s| !s.is_empty())
                .map(|s| s.trim_end_matches('/').to_string())
                .collect();

            let protected_paths: Vec<String> = config
                .protected
                .paths
                .iter()
                .filter_map(|p| {
                    let path_str = p.to_string_lossy().to_string();
                    let normalized = path_str.trim_end_matches('/');
                    if current_paths.contains(normalized) {
                        None
                    } else {
                        Some(path_str)
                    }
                })
                .collect();

            if !protected_paths.is_empty() {
                // Silently insert protected paths at the beginning
                path_var = format!("{}:{}", protected_paths.join(":"), path_var);
            }
        }
    }

    let result = match shell_opt {
        None => {
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
            println!(
                "Applied PATH to {} ({} entries)",
                shell.as_str(),
                num_entries
            );
            0
        }
        Some(shell_str) => {
            if shell_str.to_lowercase() == "all" {
                let shells = [Shell::Bash, Shell::Zsh, Shell::Fish];
                let mut all_ok = true;

                for shell in &shells {
                    if let Err(e) = save_path(shell, &path_var) {
                        eprintln!("Error applying to {}: {e}", shell.as_str());
                        all_ok = false;
                    } else {
                        let num_entries = path_var.split(':').filter(|s| !s.is_empty()).count();
                        println!(
                            "Applied PATH to {} ({} entries)",
                            shell.as_str(),
                            num_entries
                        );
                    }
                }

                if all_ok {
                    0
                } else {
                    2
                }
            } else {
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
                println!(
                    "Applied PATH to {} ({} entries)",
                    shell.as_str(),
                    num_entries
                );
                0
            }
        }
    };

    if result == 0 {
        match history_for_current_scope() {
            Ok(history) => {
                if let Err(e) = history.reset_with_initial(&path_var) {
                    eprintln!("Warning: Failed to reinitialize history: {e}");
                }

                if history.scope() == HistoryScope::Global {
                    let _ = cleanup_old_sessions();
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to update history: {e}");
            }
        }
    }

    result
}

fn handle_diff(full: bool) -> i32 {
    use crate::path_diff::{compute_diff, format_diff_with_limit};

    let current_path = env::var("PATH").unwrap_or_default();
    let use_color = atty::is(atty::Stream::Stdout);

    let baseline_path = history_for_current_scope()
        .ok()
        .and_then(|history| history.initial_snapshot().ok().flatten())
        .unwrap_or_else(|| current_path.clone());

    let diff = compute_diff(&current_path, &baseline_path, full);
    let formatted = format_diff_with_limit(&diff, use_color, full);

    println!("{formatted}");

    0
}

fn handle_reset() -> i32 {
    use std::io::Write;

    match history_for_current_scope() {
        Ok(history) => match history.initial_snapshot() {
            Ok(Some(initial_path)) => {
                if let Err(e) = history.truncate(1) {
                    eprintln!("Warning: Failed to truncate snapshot history: {e}");
                }

                if let Err(e) = history.clear_cursor() {
                    eprintln!("Warning: Failed to reset history cursor: {e}");
                }

                let stdout = io::stdout();
                let mut out = BufWriter::new(stdout.lock());
                writeln!(out, "{initial_path}").ok();
                out.flush().ok();
                0
            }
            Ok(None) => {
                eprintln!(
                    "Error: No initial PATH found. No operations have been performed in this session."
                );
                1
            }
            Err(e) => {
                eprintln!("Error: {e}");
                2
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn handle_undo(count: usize) -> i32 {
    use std::io::Write;

    if count == 0 {
        eprintln!("Error: Count must be at least 1");
        return 2;
    }

    match history_for_current_scope() {
        Ok(history) => match history.read_snapshots() {
            Ok(snapshots) => {
                if snapshots.is_empty() {
                    eprintln!(
                        "Error: No PATH history found. No operations have been performed in this session."
                    );
                    return 1;
                }

                let current_pos = match history.get_cursor() {
                    Ok(Some(pos)) => pos,
                    Ok(None) => snapshots.len() - 1,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        return 2;
                    }
                };

                if current_pos < count {
                    if current_pos == 0 {
                        eprintln!("Error: Cannot undo further. Already at initial PATH state.");
                    } else {
                        eprintln!(
                            "Error: Can only undo {current_pos} more step(s). Use 'whi reset' to go back to the initial state."
                        );
                    }
                    return 1;
                }

                let target_index = current_pos - count;
                let target_snapshot = &snapshots[target_index];

                if let Err(e) = history.set_cursor(target_index) {
                    eprintln!("Error: Failed to set cursor: {e}");
                    return 2;
                }

                let stdout = io::stdout();
                let mut out = BufWriter::new(stdout.lock());
                writeln!(out, "{target_snapshot}").ok();
                out.flush().ok();
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                2
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn handle_redo(count: usize) -> i32 {
    use std::io::Write;

    if count == 0 {
        eprintln!("Error: Count must be at least 1");
        return 2;
    }

    match history_for_current_scope() {
        Ok(history) => match history.read_snapshots() {
            Ok(snapshots) => {
                if snapshots.is_empty() {
                    eprintln!("Error: No PATH history found. No operations have been performed in this session.");
                    return 1;
                }

                let current_pos = match history.get_cursor() {
                    Ok(Some(pos)) => pos,
                    Ok(None) => {
                        eprintln!("Error: Already at the latest state. Nothing to redo.");
                        return 1;
                    }
                    Err(e) => {
                        eprintln!("Error: {e}");
                        return 2;
                    }
                };

                let max_pos = snapshots.len() - 1;
                if current_pos + count > max_pos {
                    let available = max_pos - current_pos;
                    if available == 0 {
                        eprintln!("Error: Already at the latest state. Nothing to redo.");
                    } else {
                        eprintln!("Error: Can only redo {available} more step(s).");
                    }
                    return 1;
                }

                let target_index = current_pos + count;
                let target_snapshot = &snapshots[target_index];

                if target_index == max_pos {
                    if let Err(e) = history.clear_cursor() {
                        eprintln!("Error: Failed to clear cursor: {e}");
                        return 2;
                    }
                } else if let Err(e) = history.set_cursor(target_index) {
                    eprintln!("Error: Failed to set cursor: {e}");
                    return 2;
                }

                let stdout = io::stdout();
                let mut out = BufWriter::new(stdout.lock());
                writeln!(out, "{target_snapshot}").ok();
                out.flush().ok();
                0
            }
            Err(e) => {
                eprintln!("Error: {e}");
                2
            }
        },
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn handle_save_profile(profile_name: &str) -> i32 {
    use crate::config_manager::save_profile;

    let path_var = env::var("PATH").unwrap_or_default();

    match save_profile(profile_name, &path_var) {
        Ok(()) => {
            let num_entries = path_var.split(':').filter(|s| !s.is_empty()).count();
            println!("Saved profile '{profile_name}' ({num_entries} entries)");
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            2
        }
    }
}

fn handle_load_profile(profile_name: &str) -> i32 {
    use crate::config_manager::load_profile;
    use std::io::Write;

    match load_profile(profile_name) {
        Ok(parsed) => {
            use crate::path_file::apply_path_sections;

            // Get current PATH to use as base for prepend/append
            let current_path = env::var("PATH").unwrap_or_default();

            // Apply PATH sections
            let mut path_string = match apply_path_sections(&current_path, &parsed.path) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Error applying profile: {e}");
                    return 2;
                }
            };
            // Self-protection: ensure current whi directory is in PATH (silently append if missing)
            if let Some(exe_dir) = get_current_exe_dir() {
                let canonical_exe_dir =
                    fs::canonicalize(&exe_dir).unwrap_or_else(|_| exe_dir.clone());

                // Check if exe_dir is already in the loaded PATH
                let path_entries: Vec<&str> = path_string.split(':').collect();
                let mut found = false;

                for entry in &path_entries {
                    let entry_path = PathBuf::from(entry);
                    let canonical_entry =
                        fs::canonicalize(&entry_path).unwrap_or_else(|_| entry_path.clone());

                    if entry_path == exe_dir
                        || entry_path == canonical_exe_dir
                        || canonical_entry == exe_dir
                        || canonical_entry == canonical_exe_dir
                    {
                        found = true;
                        break;
                    }
                }

                // If not found, append it
                if !found {
                    if !path_string.is_empty() {
                        path_string.push(':');
                    }
                    path_string.push_str(&exe_dir.display().to_string());
                }
            }

            match history_for_current_scope() {
                Ok(history) => {
                    if let Err(e) = history.write_snapshot(&path_string) {
                        eprintln!("Warning: Failed to write snapshot for loaded profile: {e}");
                    }
                }
                Err(e) => {
                    eprintln!("Warning: Failed to acquire history for loaded profile: {e}");
                }
            }

            let stdout = io::stdout();
            let mut out = BufWriter::new(stdout.lock());

            // Apply path guard to preserve critical binaries (whi, zoxide)
            let original_path = env::var("PATH").unwrap_or_default();
            let guarded_path =
                PathGuard::default().ensure_protected_paths(&original_path, path_string);

            writeln!(out, "{guarded_path}").ok();
            out.flush().ok();
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

fn handle_remove_profile(profile_name: &str) -> i32 {
    use crate::config_manager::delete_profile;

    match delete_profile(profile_name) {
        Ok(()) => {
            println!("Removed profile '{profile_name}'");
            0
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
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

        crate::system::is_tty(fd)
    }

    #[derive(Copy, Clone)]
    pub enum Stream {
        Stdout,
        Stdin,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[test]
    fn test_protected_path_normalization() {
        // Test that paths with/without trailing slashes are treated as equal
        let current = "/usr/local/sbin/:/usr/bin:/bin";
        let protected = vec![PathBuf::from("/usr/local/sbin")];

        let current_paths: HashSet<String> = current
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .collect();

        let missing: Vec<String> = protected
            .iter()
            .filter_map(|p| {
                let path_str = p.to_string_lossy().to_string();
                let normalized = path_str.trim_end_matches('/');
                if current_paths.contains(normalized) {
                    None
                } else {
                    Some(path_str)
                }
            })
            .collect();

        // Should recognize /usr/local/sbin/ matches /usr/local/sbin
        assert!(
            missing.is_empty(),
            "Expected no missing paths, found: {:?}",
            missing
        );
    }

    #[test]
    fn test_protected_path_missing() {
        // Test that missing protected paths are detected
        let current = "/usr/bin:/bin";
        let protected = vec![PathBuf::from("/usr/local/sbin")];

        let current_paths: HashSet<String> = current
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .collect();

        let missing: Vec<String> = protected
            .iter()
            .filter_map(|p| {
                let path_str = p.to_string_lossy().to_string();
                let normalized = path_str.trim_end_matches('/');
                if current_paths.contains(normalized) {
                    None
                } else {
                    Some(path_str)
                }
            })
            .collect();

        assert_eq!(missing.len(), 1, "Expected 1 missing path");
        assert_eq!(missing[0], "/usr/local/sbin");
    }

    #[test]
    fn test_protected_path_already_present() {
        // Test that existing protected paths are not duplicated
        let current = "/usr/local/sbin:/usr/bin:/bin";
        let protected = vec![PathBuf::from("/usr/local/sbin")];

        let current_paths: HashSet<String> = current
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .collect();

        let missing: Vec<String> = protected
            .iter()
            .filter_map(|p| {
                let path_str = p.to_string_lossy().to_string();
                let normalized = path_str.trim_end_matches('/');
                if current_paths.contains(normalized) {
                    None
                } else {
                    Some(path_str)
                }
            })
            .collect();

        assert!(
            missing.is_empty(),
            "Expected no missing paths when already present"
        );
    }
}
