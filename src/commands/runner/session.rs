use std::env;
use std::fs;
use std::io::{self, BufWriter};
use std::path::PathBuf;

use crate::cli::args::ApplyTarget;
use crate::commands::support::path_support::{emit_line, history_for_current_scope, output_path};
use crate::config::{protected_paths, shell_paths};
use crate::path::file::apply_path_sections;
use crate::session::store::cleanup_old_sessions;
use crate::shell::detect::{Shell, detect_current_shell};
use crate::shell::init as shell_init;

pub(super) fn handle_init(shell: &str) -> i32 {
    match shell_init::generate_init_script(shell) {
        Ok(script) => {
            print!("{script}");
            0
        }
        Err(err) => {
            eprintln!("Error: {err}");
            2
        }
    }
}

#[allow(clippy::too_many_lines)]
pub(super) fn handle_apply(apply_target: &ApplyTarget, no_protect: bool) -> i32 {
    use std::collections::HashSet;

    let mut path_var = env::var("PATH").unwrap_or_default();

    if !no_protect && let Ok(protected_path_bufs) = protected_paths::load_protected_paths() {
        let current_paths: HashSet<String> = path_var
            .split(':')
            .filter(|s| !s.is_empty())
            .map(|s| s.trim_end_matches('/').to_string())
            .collect();

        let protected_paths: Vec<String> = protected_path_bufs
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
            path_var = format!("{}:{}", protected_paths.join(":"), path_var);
        }
    }

    let result = match apply_target {
        ApplyTarget::CurrentShell => {
            let shell = match detect_current_shell() {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("Error: {e}");
                    return 2;
                }
            };

            if let Err(e) = shell_paths::save_path(&shell, &path_var) {
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
        ApplyTarget::Shell(shell_str) => {
            if shell_str.to_lowercase() == "all" {
                let shells = [Shell::Bash, Shell::Zsh, Shell::Fish];
                let mut all_ok = true;

                for shell in &shells {
                    if let Err(e) = shell_paths::save_path(shell, &path_var) {
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

                if all_ok { 0 } else { 2 }
            } else {
                let shell = match shell_str.parse::<Shell>() {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Error: {e}");
                        return 2;
                    }
                };

                if let Err(e) = shell_paths::save_path(&shell, &path_var) {
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

                if let Err(e) = cleanup_old_sessions() {
                    eprintln!("Warning: Failed to clean up old sessions: {e}");
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to update history: {e}");
            }
        }
    }

    result
}

pub(super) fn handle_diff(full: bool) -> i32 {
    let current_path = env::var("PATH").unwrap_or_default();
    let use_color = super::atty::is(super::atty::Stream::Stdout);

    let baseline_path = history_for_current_scope()
        .ok()
        .and_then(|history| history.initial_snapshot().ok().flatten())
        .unwrap_or_else(|| current_path.clone());

    let diff = crate::path::diff::compute_diff(&current_path, &baseline_path, full);
    let formatted = crate::path::diff::format_diff_with_limit(&diff, use_color, full);

    println!("{formatted}");

    0
}

pub(super) fn handle_reset() -> i32 {
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
                emit_line(&mut out, &initial_path)
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

pub(super) fn handle_undo(count: usize) -> i32 {
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
                emit_line(&mut out, target_snapshot)
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

pub(super) fn handle_redo(count: usize) -> i32 {
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
                emit_line(&mut out, target_snapshot)
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

pub(super) fn handle_save_profile(profile_name: &str) -> i32 {
    let path_var = env::var("PATH").unwrap_or_default();

    match shell_paths::save_profile(profile_name, &path_var) {
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

pub(super) fn handle_load_profile(profile_name: &str) -> i32 {
    match shell_paths::load_profile(profile_name) {
        Ok(parsed) => {
            let current_path = env::var("PATH").unwrap_or_default();

            let mut path_string = match apply_path_sections(&current_path, &parsed.path) {
                Ok(path) => path,
                Err(e) => {
                    eprintln!("Error applying profile: {e}");
                    return 2;
                }
            };

            if let Some(exe_dir) = super::get_current_exe_dir() {
                let canonical_exe_dir =
                    fs::canonicalize(&exe_dir).unwrap_or_else(|_| exe_dir.clone());
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
            output_path(&mut out, &path_string)
        }
        Err(e) => {
            eprintln!("Error: {e}");
            1
        }
    }
}

pub(super) fn handle_remove_profile(profile_name: &str) -> i32 {
    match shell_paths::delete_profile(profile_name) {
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
