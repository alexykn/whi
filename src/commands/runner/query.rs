use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::io::{self, BufRead, BufWriter, StdoutLock, Write};
use std::path::{Path, PathBuf};

use crate::cli::args::Args;
use crate::config::runtime::Config;
use crate::io::output::OutputFormatter;
use crate::path::fuzzy::FuzzyMatcher;
use crate::path::searcher::PathSearcher;
use crate::search::result::{ExecutableCheck, SearchResult};

pub(super) use crate::commands::support::path_support::{
    output_path, should_use_color, write_snapshot_safe,
};

pub(super) fn run_query(
    searcher: &PathSearcher,
    args: &Args,
    config: &Config,
    out: &mut BufWriter<StdoutLock<'_>>,
) -> i32 {
    let names = get_names(args);

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
    let stderr = io::stderr();
    let mut err = BufWriter::new(stderr.lock());

    let use_color = should_use_color(args, super::atty::is(super::atty::Stream::Stdout));
    let mut formatter = OutputFormatter::new(use_color, args.print0);

    for name in names {
        let use_fuzzy = config.search.executable_search_fuzzy ^ args.swap_fuzzy;

        let results = if !name.contains('/') && use_fuzzy {
            search_name_fuzzy(searcher, &name, args)
        } else {
            search_name(searcher, &name, args)
        };

        if results.is_empty() {
            all_found = false;

            if !args.silent && !args.quiet {
                writeln!(err, "{name}: not found").ok();
            }
            continue;
        }

        let max_index = results.iter().map(|r| r.path_index).max().unwrap_or(0);
        if max_index > 999 {
            if !args.silent {
                eprintln!("Error: PATH index {max_index} exceeds max 999");
            }
            return 3;
        }

        if !name.contains('/') && use_fuzzy {
            let mut by_index: BTreeMap<usize, Vec<&SearchResult>> = BTreeMap::new();
            for result in &results {
                by_index.entry(result.path_index).or_default().push(result);
            }

            for index_results in by_index.values_mut() {
                index_results.sort_by(|a, b| {
                    let name_a = a.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    let name_b = b.path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                    name_a.cmp(name_b)
                });
            }

            let mut seen_names: HashSet<String> = HashSet::new();

            for index_results in by_index.into_values() {
                for result in &index_results {
                    let file_name = result
                        .path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");

                    let is_winner = seen_names.insert(file_name.to_string());

                    if !args.all && !args.full && !is_winner {
                        continue;
                    }

                    formatter
                        .write_result(
                            out,
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
            for (i, result) in results.iter().enumerate() {
                let is_winner = i == 0;

                formatter
                    .write_result(
                        out,
                        result,
                        is_winner,
                        args.follow_symlinks,
                        !args.no_index,
                        3,
                    )
                    .ok();

                if (!args.all && !args.full) || args.one {
                    break;
                }
            }
        }

        if args.full {
            writeln!(out).ok();

            let match_indices: HashSet<usize> = results.iter().map(|r| r.path_index).collect();

            for (idx, dir) in searcher.dirs().iter().enumerate() {
                let path_index = idx + 1;
                let has_match = match_indices.contains(&path_index);

                if !args.no_index {
                    write!(out, "{:>4} ", format!("[{}]", path_index)).ok();
                }

                if use_color && has_match {
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

pub(super) fn get_names(args: &Args) -> Vec<String> {
    if !args.names.is_empty() {
        return args.names.clone();
    }

    if !super::atty::is(super::atty::Stream::Stdin) {
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

    Vec::new()
}

pub(super) fn search_name(searcher: &PathSearcher, name: &str, args: &Args) -> Vec<SearchResult> {
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

            if !search_all {
                break;
            }
        }
    }

    results
}

fn check_dir_entry(entry: &fs::DirEntry, args: &Args, path_index: usize) -> Option<SearchResult> {
    let path = entry.path();
    let metadata = fs::metadata(&path).ok()?;

    if !metadata.is_file() && !args.show_nonexec {
        return None;
    }

    let checker = ExecutableCheck::with_metadata(&path, metadata.clone());

    if !checker.is_executable() && !args.show_nonexec {
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

fn search_name_fuzzy(searcher: &PathSearcher, query: &str, args: &Args) -> Vec<SearchResult> {
    use std::ffi::OsStr;

    let matcher = FuzzyMatcher::new(query);
    let mut results = Vec::new();

    for (idx, dir) in searcher.dirs().iter().enumerate() {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(filename) = path.file_name().and_then(OsStr::to_str) else {
                continue;
            };

            if matcher.matches(&PathBuf::from(filename))
                && let Some(result) = check_dir_entry(&entry, args, idx + 1)
            {
                results.push(result);
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

    if !checker.is_executable() && !args.show_nonexec {
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
