use crate::executor::SearchResult;
use std::io::Write;
use std::time::SystemTime;

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
                writeln!(out, "  modified: {}", Self::format_time(mtime))?;
            }
        }

        Ok(())
    }

    fn format_time(time: SystemTime) -> String {
        use std::time::UNIX_EPOCH;

        let duration = time.duration_since(UNIX_EPOCH).unwrap_or_default();
        let secs = duration.as_secs();

        // Convert to US date/time format: MM/DD/YYYY HH:MM:SS AM/PM
        let days_since_epoch = secs / 86400;
        let seconds_today = secs % 86400;

        // Calculate date (simplified epoch calculation)
        // Days since Unix epoch (Jan 1, 1970)
        let mut year = 1970;
        let mut days_left = days_since_epoch;

        // Adjust for years
        loop {
            let days_in_year = if Self::is_leap_year(year) { 366 } else { 365 };
            if days_left < days_in_year {
                break;
            }
            days_left -= days_in_year;
            year += 1;
        }

        // Find month and day
        let days_in_months = if Self::is_leap_year(year) {
            [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        } else {
            [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
        };

        let mut month = 1;
        let mut day = days_left + 1;
        for &days_in_month in &days_in_months {
            if day <= days_in_month as u64 {
                break;
            }
            day -= days_in_month as u64;
            month += 1;
        }

        // Calculate time
        let hour = (seconds_today / 3600) as u32;
        let minute = ((seconds_today % 3600) / 60) as u32;
        let second = (seconds_today % 60) as u32;

        format!(
            "{}-{:02}-{:02} {:02}:{:02}:{:02}",
            year, month, day, hour, minute, second
        )
    }

    fn is_leap_year(year: u64) -> bool {
        (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
    }
}
