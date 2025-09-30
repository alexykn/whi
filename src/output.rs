use crate::executor::SearchResult;
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
        show_index: bool,
    ) -> std::io::Result<()> {
        // Show index if requested
        if show_index {
            write!(out, "[{}] ", result.path_index)?;
        }

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
