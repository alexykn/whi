use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub enum DiffEntry {
    Added(String),     // + entry (new in current)
    Removed(String),   // - entry (gone from initial)
    Moved(String),     // M entry (position changed)
    Unchanged(String), // U entry (same position, only in full mode)
}

#[derive(Debug)]
pub struct PathDiff {
    pub entries: Vec<DiffEntry>,
}

impl PathDiff {
    #[allow(dead_code)]
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries
            .iter()
            .all(|e| matches!(e, DiffEntry::Unchanged(_)))
    }
}

/// Compute diff between current and initial `PATH`
/// Simply compares current state to initial snapshot - no operation tracking needed!
/// This means diff will show `ALL` changes, including manual `export PATH=...` modifications
pub fn compute_diff(current: &str, initial: &str, _full: bool) -> PathDiff {
    let current_entries: Vec<String> = current
        .split(':')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let initial_entries: Vec<String> = initial
        .split(':')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    // Build position maps (first occurrence only)
    let mut initial_positions: HashMap<String, usize> = HashMap::new();
    for (idx, entry) in initial_entries.iter().enumerate() {
        initial_positions.entry(entry.clone()).or_insert(idx);
    }

    let mut current_positions: HashMap<String, usize> = HashMap::new();
    for (idx, entry) in current_entries.iter().enumerate() {
        current_positions.entry(entry.clone()).or_insert(idx);
    }

    // Build sets for membership testing
    let initial_set: std::collections::HashSet<String> = initial_entries.iter().cloned().collect();
    let current_set: std::collections::HashSet<String> = current_entries.iter().cloned().collect();

    let mut diff_entries = Vec::new();

    // Process removals (entries in initial but not in current)
    for entry in &initial_entries {
        if !current_set.contains(entry) {
            diff_entries.push(DiffEntry::Removed(entry.clone()));
        }
    }

    // Process current entries in order
    for entry in &current_entries {
        // New entry
        if !initial_set.contains(entry) {
            diff_entries.push(DiffEntry::Added(entry.clone()));
            continue;
        }

        // Entry exists in both - check if position changed
        let initial_pos = initial_positions[entry];
        let current_pos = current_positions[entry];

        if initial_pos == current_pos {
            // Same position - unchanged
            diff_entries.push(DiffEntry::Unchanged(entry.clone()));
        } else {
            // Position changed - show as moved
            diff_entries.push(DiffEntry::Moved(entry.clone()));
        }
    }

    PathDiff {
        entries: diff_entries,
    }
}

/// Format the diff for display
#[must_use]
pub fn format_diff(diff: &PathDiff, use_color: bool) -> String {
    format_diff_with_limit(diff, use_color, false)
}

/// Format the diff for display with optional entry limit
#[must_use]
pub fn format_diff_with_limit(diff: &PathDiff, use_color: bool, full: bool) -> String {
    const MAX_ENTRIES: usize = 15;

    // Check if there are any actual changes
    let has_changes = diff
        .entries
        .iter()
        .any(|e| !matches!(e, DiffEntry::Unchanged(_)));

    if !has_changes {
        return "No differences".to_string();
    }

    let mut output = Vec::new();

    // Colors
    let (red, green, cyan, gray, reset) = if use_color {
        ("\x1b[31m", "\x1b[32m", "\x1b[36m", "\x1b[90m", "\x1b[0m")
    } else {
        ("", "", "", "", "")
    };

    // Count entry types
    let mut added = 0;
    let mut removed = 0;
    let mut moved = 0;
    let mut unchanged = 0;

    for entry in &diff.entries {
        match entry {
            DiffEntry::Added(_) => added += 1,
            DiffEntry::Removed(_) => removed += 1,
            DiffEntry::Moved(_) => moved += 1,
            DiffEntry::Unchanged(_) => unchanged += 1,
        }
    }

    // Build summary line
    let mut summary_parts = Vec::new();
    if added > 0 {
        summary_parts.push(format!("{green}+{added}{reset}"));
    }
    if removed > 0 {
        summary_parts.push(format!("{red}-{removed}{reset}"));
    }
    if moved > 0 {
        summary_parts.push(format!("{cyan}M{moved}{reset}"));
    }
    if unchanged > 0 {
        summary_parts.push(format!("{gray}U{unchanged}{reset}"));
    }

    if !summary_parts.is_empty() {
        output.push(summary_parts.join(" | "));
        output.push(String::new()); // Blank line
    }

    // Separate removals from current PATH entries
    let mut removal_lines = Vec::new();
    let mut current_path_lines = Vec::new();

    for entry in &diff.entries {
        match entry {
            DiffEntry::Removed(path) => {
                removal_lines.push(format!("{red}- {path}{reset}"));
            }
            DiffEntry::Added(path) => {
                current_path_lines.push(format!("{green}+ {path}{reset}"));
            }
            DiffEntry::Moved(path) => {
                current_path_lines.push(format!("{cyan}M {path}{reset}"));
            }
            DiffEntry::Unchanged(path) => {
                current_path_lines.push(format!("{gray}U {path}{reset}"));
            }
        }
    }

    // Output removals (always show all)
    output.extend(removal_lines);

    // Output current PATH entries (with truncation in non-full mode)
    let total_current = current_path_lines.len();
    if !full && total_current > MAX_ENTRIES {
        output.extend(current_path_lines.into_iter().take(MAX_ENTRIES));
        let remaining = total_current - MAX_ENTRIES;
        output.push(format!(
            "{gray}... and {remaining} more entries. Run 'whi diff full' to see all.{reset}"
        ));
    } else {
        output.extend(current_path_lines);
    }

    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff_no_changes() {
        let initial = "/a:/b:/c";
        let current = "/a:/b:/c";

        let diff = compute_diff(current, initial, false);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_compute_diff_addition() {
        let initial = "/a:/b";
        let current = "/a:/b:/c";

        let diff = compute_diff(current, initial, false);
        // Should show: /a unchanged, /b unchanged, /c added
        assert_eq!(diff.entries.len(), 3);
        assert!(matches!(diff.entries[0], DiffEntry::Unchanged(_)));
        assert!(matches!(diff.entries[1], DiffEntry::Unchanged(_)));
        assert!(matches!(diff.entries[2], DiffEntry::Added(_)));
    }

    #[test]
    fn test_compute_diff_removal() {
        let initial = "/a:/b:/c";
        let current = "/a:/b";

        let diff = compute_diff(current, initial, false);
        // Should show: /c removed, /a unchanged, /b unchanged
        assert_eq!(diff.entries.len(), 3);
        assert!(matches!(diff.entries[0], DiffEntry::Removed(_)));
        assert!(matches!(diff.entries[1], DiffEntry::Unchanged(_)));
        assert!(matches!(diff.entries[2], DiffEntry::Unchanged(_)));
    }

    #[test]
    fn test_compute_diff_move() {
        let initial = "/a:/b:/c";
        let current = "/c:/a:/b";

        let diff = compute_diff(current, initial, false);
        // All three moved positions
        assert_eq!(
            diff.entries
                .iter()
                .filter(|e| matches!(e, DiffEntry::Moved(_)))
                .count(),
            3
        );
    }

    #[test]
    fn test_compute_diff_full_mode() {
        let initial = "/a:/b:/c";
        let current = "/c:/a:/b";

        let diff = compute_diff(current, initial, true);
        // Should show all 3 as moved (no unchanged since all positions changed)
        assert_eq!(diff.entries.len(), 3);
        assert!(diff
            .entries
            .iter()
            .all(|e| matches!(e, DiffEntry::Moved(_))));
    }

    #[test]
    fn test_compute_diff_mixed_changes() {
        let initial = "/a:/b:/c";
        let current = "/d:/a:/c"; // removed /b, added /d, /a moved, /c same position

        let diff = compute_diff(current, initial, false);

        // Should have: -/b, +/d, M/a
        // Note: /c stays at position 2 in both, so no change shown (unless full mode)
        assert!(diff
            .entries
            .iter()
            .any(|e| matches!(e, DiffEntry::Removed(p) if p == "/b")));
        assert!(diff
            .entries
            .iter()
            .any(|e| matches!(e, DiffEntry::Added(p) if p == "/d")));
        assert!(diff
            .entries
            .iter()
            .any(|e| matches!(e, DiffEntry::Moved(p) if p == "/a")));
    }

    #[test]
    fn test_tracks_manual_export() {
        // User manually does: export PATH="/new:$PATH"
        let initial = "/a:/b:/c";
        let current = "/new:/a:/b:/c";

        let diff = compute_diff(current, initial, false);

        // Should show /new as added (even though whi didn't do it!)
        assert!(diff
            .entries
            .iter()
            .any(|e| matches!(e, DiffEntry::Added(p) if p == "/new")));
    }

    #[test]
    fn test_unchanged_in_full_mode() {
        let initial = "/a:/b:/c";
        let current = "/a:/b:/c";

        let diff = compute_diff(current, initial, true);

        // In full mode, should show all as unchanged
        assert_eq!(diff.entries.len(), 3);
        assert!(diff
            .entries
            .iter()
            .all(|e| matches!(e, DiffEntry::Unchanged(_))));
    }
}
