use crate::cli::Args;
use crate::executor::SearchResult;
use crate::path::PathSearcher;
use std::io::Write;

pub struct OutputFormatter {
    use_color: bool,
    print0: bool,
}

impl OutputFormatter {
    pub fn new(use_color: bool, print0: bool) -> Self {
        OutputFormatter { use_color, print0 }
    }

    pub fn write_result<W: Write>(
        &mut self,
        out: &mut W,
        result: &SearchResult,
        is_winner: bool,
        follow_symlinks: bool,
        show_status: bool,
    ) -> std::io::Result<()> {
        let path_str = result.path.display().to_string();

        if self.use_color && is_winner {
            write!(out, "\x1b[1;32m{path_str}\x1b[0m")?;
        } else {
            write!(out, "{path_str}")?;
        }

        // If following symlinks, show canonical path
        if follow_symlinks {
            if let Some(ref canonical) = result.canonical_path {
                if canonical != &result.path {
                    write!(out, " → {}", canonical.display())?;
                }
            }
        }

        // Show status marker only if requested (with -e flag)
        if show_status {
            let status = if is_winner {
                if self.use_color {
                    " \x1b[1;32m(winner)\x1b[0m"
                } else {
                    " (winner)"
                }
            } else {
                " (shadowed)"
            };
            write!(out, "{status}")?;
        }

        if self.print0 {
            write!(out, "\0")?;
        } else {
            writeln!(out)?;
        }

        // Show metadata if present (works with or without -e)
        if let Some(ref meta) = result.metadata {
            writeln!(
                out,
                "  inode: {}, device: {}, size: {} bytes",
                meta.ino, meta.dev, meta.size
            )?;
            if let Some(mtime) = meta.mtime {
                writeln!(out, "  modified: {mtime:?}")?;
            }
        }

        Ok(())
    }
}

pub struct ExplainFormatter {
    use_color: bool,
}

impl ExplainFormatter {
    pub fn new(use_color: bool) -> Self {
        ExplainFormatter { use_color }
    }

    pub fn write_explanation<W: Write>(
        &self,
        err: &mut W,
        name: &str,
        searcher: &PathSearcher,
        results: &[SearchResult],
        args: &Args,
    ) -> std::io::Result<()> {
        writeln!(err)?;

        // Header with name
        writeln!(err, "[whicha] {name}")?;

        // If -f/--full, show full PATH listing first
        if args.full {
            writeln!(err)?;
            let total_dirs = searcher.dirs().len();
            writeln!(
                err,
                "PATH ({} {})",
                total_dirs,
                if total_dirs == 1 { "entry" } else { "entries" }
            )?;

            for (idx, dir) in searcher.dirs().iter().enumerate() {
                writeln!(err, "  [{}] {}", idx + 1, dir.display())?;
            }
            writeln!(err)?;
        }

        // Found hits section
        if results.is_empty() {
            writeln!(err, "  not found in PATH")?;
        } else {
            if args.full {
                writeln!(err, "Found:")?;
            }

            for (i, result) in results.iter().enumerate() {
                let is_winner = i == 0;
                let status = if is_winner {
                    if self.use_color {
                        "\x1b[1;32m(winner)\x1b[0m"
                    } else {
                        "(winner)"
                    }
                } else {
                    "(shadowed)"
                };

                let path_display = result.path.display();
                write!(err, "  [{}] {} {}", result.path_index, path_display, status)?;

                // Add canonical path if following symlinks
                if args.follow_symlinks {
                    if let Some(ref canonical) = result.canonical_path {
                        if canonical != &result.path {
                            write!(err, " → {}", canonical.display())?;
                        }
                    }
                }

                writeln!(err)?;

                // Additional details
                if !result.is_executable {
                    writeln!(err, "       (exists but not executable)")?;
                }

                if let Some(ref meta) = result.metadata {
                    writeln!(
                        err,
                        "       inode: {}, device: {}, size: {} bytes",
                        meta.ino, meta.dev, meta.size
                    )?;
                    if let Some(mtime) = meta.mtime {
                        writeln!(err, "       modified: {mtime:?}")?;
                    }
                }
            }

            // Winner explanation
            writeln!(err)?;
            writeln!(
                err,
                "Winner: earliest PATH hit is [#{}].",
                results[0].path_index
            )?;

            // Notes section (only with --full)
            if args.full {
                writeln!(err)?;
                writeln!(
                    err,
                    "Note: shell aliases/functions/builtins aren't visible to whicha."
                )?;
                writeln!(
                    err,
                    "      Use 'type -a {name}' in your shell to see all matches."
                )?;
            }
        }

        writeln!(err)?;

        Ok(())
    }
}
