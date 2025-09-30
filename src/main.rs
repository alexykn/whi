use std::env;
use std::fs;
use std::io::{self, BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process;

mod cli;
mod executor;
mod output;
mod path;

use cli::{Args, ColorWhen};
use executor::{ExecutableCheck, SearchResult};
use output::{ExplainFormatter, OutputFormatter};
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

fn run(args: &Args) -> i32 {
    let names = get_names(args);
    if names.is_empty() {
        if !args.silent {
            eprintln!("Error: no names provided");
        }
        return 2;
    }

    let path_var = match &args.path_override {
        Some(p) => p.clone(),
        None => env::var("PATH").unwrap_or_default(),
    };

    let searcher = PathSearcher::new(&path_var);
    let mut all_found = true;
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let stderr = io::stderr();
    let mut err = BufWriter::new(stderr.lock());

    let use_color = should_use_color(args);
    let mut formatter = OutputFormatter::new(use_color, args.print0);

    for name in names {
        let results = search_name(&searcher, &name, args);

        if results.is_empty() {
            all_found = false;

            if args.explain && !args.silent {
                let explain = ExplainFormatter::new(use_color);
                explain
                    .write_explanation(&mut err, &name, &searcher, &results, args)
                    .ok();
            } else if !args.silent && !args.quiet {
                writeln!(err, "{name}: not found").ok();
            }
            continue;
        }

        if args.explain {
            let explain = ExplainFormatter::new(use_color);
            explain
                .write_explanation(&mut err, &name, &searcher, &results, args)
                .ok();
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
                    args.explain,
                )
                .ok();

            if args.first {
                break;
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

    // Read from stdin
    let stdin = io::stdin();
    let mut names = Vec::new();
    for line in stdin.lock().lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with('#') {
            names.push(trimmed.to_string());
        }
    }
    names
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
        is_executable,
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

// Simple atty implementation using /dev/tty
mod atty {
    use std::fs::File;

    pub fn is(_: Stream) -> bool {
        // Try to open /dev/tty - if it works, we're probably connected to a terminal
        File::open("/dev/tty").is_ok()
    }

    pub enum Stream {
        Stdout,
    }
}
