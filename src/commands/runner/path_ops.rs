use std::env;
use std::fs;
use std::io::{BufWriter, StdoutLock};
use std::path::{Path, PathBuf};

use crate::cli::args::{Args, DeleteTarget, PathEdit, PreferTarget};
use crate::commands::support::path_support::{
    emit_line, output_path, should_use_color, write_snapshot_safe,
};
use crate::path::resolve::{looks_like_exact_path, resolve_path};
use crate::path::searcher::PathSearcher;

use super::handle_path_result;
use super::query::search_name;

pub(super) fn handle_clean(
    searcher: &PathSearcher,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    let (new_path, _removed_indices) = searcher.clean_duplicates();
    write_snapshot_safe(&new_path, args);
    output_path(out, &new_path)
}

pub(super) fn handle_move_or_swap(
    searcher: &PathSearcher,
    path_edit: &PathEdit,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    let result = match path_edit {
        PathEdit::Move { from, to } => searcher.move_entry(*from, *to),
        PathEdit::Swap { first, second } => searcher.swap_entries(*first, *second),
    };
    handle_path_result(result, args, out)
}

pub(super) fn handle_prefer(
    searcher: &PathSearcher,
    target: &PreferTarget,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
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

fn handle_prefer_index(
    searcher: &PathSearcher,
    name: &str,
    target_idx: usize,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    let mut search_args = args.clone();
    search_args.all = true;
    let results = search_name(searcher, name, &search_args);

    if results.is_empty() {
        if !args.silent {
            eprintln!("Error: {name}: not found");
        }
        return 1;
    }

    let winner_idx = results[0].path_index;
    let target_result = results.iter().find(|r| r.path_index == target_idx);
    if target_result.is_none() {
        if !args.silent {
            eprintln!("Error: {name} not found at index {target_idx}");
        }
        return 2;
    }

    let new_position = if target_idx > winner_idx {
        winner_idx
    } else {
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

fn handle_prefer_path(
    searcher: &PathSearcher,
    name: &str,
    path_str: &str,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if looks_like_exact_path(path_str) {
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
        handle_prefer_fuzzy(searcher, name, path_str, args, out)
    }
}

fn handle_prefer_exact_path(
    searcher: &PathSearcher,
    name: &str,
    path: &Path,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    if !path.exists() {
        if !args.silent {
            eprintln!("Error: Directory does not exist: {}", path.display());
        }
        return 2;
    }

    if let Some(idx) = searcher.find_path_index(path) {
        return handle_prefer_index(searcher, name, idx, args, out);
    }

    if !searcher.has_executable(path, name) {
        if !args.silent {
            eprintln!("Error: {} not found in {}", name, path.display());
        }
        return 2;
    }

    let results = search_name(searcher, name, args);

    let insert_position = if results.is_empty() {
        1
    } else {
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
            output_path(out, &new_path)
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error adding to PATH: {e}");
            }
            2
        }
    }
}

fn handle_prefer_path_only(
    searcher: &PathSearcher,
    path_str: &str,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

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
        cwd.join(path_str)
    };

    if let Some(_idx) = searcher.find_path_index(&resolved_path) {
        if !args.silent {
            eprintln!("{} is already in PATH", resolved_path.display());
        }
        return emit_line(out, &searcher.to_path_string());
    }

    match searcher.add_path(&resolved_path) {
        Ok((new_path, idx)) => {
            if !args.silent {
                eprintln!("Added {} to PATH at index {}", resolved_path.display(), idx);
            }

            write_snapshot_safe(&new_path, args);
            output_path(out, &new_path)
        }
        Err(e) => {
            if !args.silent {
                eprintln!("Error adding to PATH: {e}");
            }
            2
        }
    }
}

fn handle_prefer_fuzzy(
    searcher: &PathSearcher,
    name: &str,
    pattern: &str,
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
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

    let (index, _) = matches[0];
    handle_prefer_index(searcher, name, index, args, out)
}

pub(super) fn handle_delete(
    searcher: &PathSearcher,
    targets: &[DeleteTarget],
    args: &Args,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut indices_to_delete = Vec::new();

    for target in targets {
        match target {
            DeleteTarget::Index(idx) => {
                indices_to_delete.push(*idx);
            }
            DeleteTarget::Path(path_str) => {
                if looks_like_exact_path(path_str) {
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
                    let matches = searcher.find_fuzzy_indices(path_str, None);

                    if matches.is_empty() {
                        if !args.silent {
                            eprintln!("Error: No PATH entries match pattern '{path_str}'");
                        }
                        return 1;
                    }

                    for (idx, _) in &matches {
                        indices_to_delete.push(*idx);
                    }
                }
            }
        }
    }

    let dirs = searcher.dirs();

    if let Some(exe_dir) = super::get_current_exe_dir() {
        let canonical_exe_dir = fs::canonicalize(&exe_dir).unwrap_or_else(|_| exe_dir.clone());

        indices_to_delete.retain(|&idx| {
            if idx > 0 && idx <= dirs.len() {
                let path = &dirs[idx - 1];
                let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());

                path != &exe_dir
                    && path != &canonical_exe_dir
                    && canonical_path != exe_dir
                    && canonical_path != canonical_exe_dir
            } else {
                true
            }
        });
    }

    indices_to_delete.sort_unstable();
    indices_to_delete.dedup();

    if !args.silent && !indices_to_delete.is_empty() {
        let use_color = should_use_color(args, super::atty::is(super::atty::Stream::Stdout));
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
