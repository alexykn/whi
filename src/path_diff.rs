use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq)]
pub enum DiffEntry {
    Added(String),         // + entry (new in current)
    Removed(String),       // - entry (gone from saved)
    MovedExplicit(String), // ↑/↓ entry (explicitly moved by user)
    MovedImplicit(String), // M entry (implicitly shifted, only in full mode)
    Unchanged(String),     // U entry (same position, only in full mode)
}

#[derive(Debug)]
pub struct PathDiff {
    pub entries: Vec<DiffEntry>,
}

impl PathDiff {
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries
            .iter()
            .all(|e| matches!(e, DiffEntry::Unchanged(_)))
    }
}

/// Compute diff between current and saved PATH
/// affected_paths: Set of paths that were explicitly affected by user operations
/// deleted_paths: List of paths that were explicitly deleted (including duplicates)
/// full: If true, shows implicit moves (M) and unchanged (U) entries
pub fn compute_diff(
    current: &str,
    saved: &str,
    affected_paths: &HashSet<String>,
    deleted_paths: &[String],
    full: bool,
) -> PathDiff {
    let current_entries: Vec<String> = current
        .split(':')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    let saved_entries: Vec<String> = saved
        .split(':')
        .filter(|s| !s.is_empty())
        .map(String::from)
        .collect();

    // Build position maps (first occurrence only)
    let mut saved_positions: HashMap<String, usize> = HashMap::new();
    for (idx, entry) in saved_entries.iter().enumerate() {
        saved_positions.entry(entry.clone()).or_insert(idx);
    }

    let mut current_positions: HashMap<String, usize> = HashMap::new();
    for (idx, entry) in current_entries.iter().enumerate() {
        current_positions.entry(entry.clone()).or_insert(idx);
    }

    // Build sets for membership testing
    let saved_set: HashSet<String> = saved_entries.iter().cloned().collect();
    let current_set: HashSet<String> = current_entries.iter().cloned().collect();

    let mut diff_entries = Vec::new();

    // Process explicitly deleted paths first (including removed duplicates)
    for path in deleted_paths {
        diff_entries.push(DiffEntry::Removed(path.clone()));
    }

    // Process removals (entries in saved but not in current)
    // Skip if already added as explicitly deleted
    for entry in &saved_entries {
        if !current_set.contains(entry) && !deleted_paths.contains(entry) {
            diff_entries.push(DiffEntry::Removed(entry.clone()));
        }
    }

    // Process current entries in order
    for entry in &current_entries {
        // New entry
        if !saved_set.contains(entry) {
            diff_entries.push(DiffEntry::Added(entry.clone()));
            continue;
        }

        // Entry exists in both - check if position changed
        let saved_pos = saved_positions[entry];
        let current_pos = current_positions[entry];

        if saved_pos != current_pos {
            // Position changed
            if affected_paths.contains(entry) {
                // Explicitly moved by user operation
                diff_entries.push(DiffEntry::MovedExplicit(entry.clone()));
            } else if full {
                // Implicitly shifted - only show in full mode
                diff_entries.push(DiffEntry::MovedImplicit(entry.clone()));
            }
            // else: not full mode and not explicit, don't show
        } else if full {
            // Same position - only show in full mode
            diff_entries.push(DiffEntry::Unchanged(entry.clone()));
        }
    }

    PathDiff {
        entries: diff_entries,
    }
}

/// Format the diff for display
pub fn format_diff(diff: &PathDiff, use_color: bool) -> String {
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
    let (red, green, cyan, reset) = if use_color {
        ("\x1b[31m", "\x1b[32m", "\x1b[36m", "\x1b[0m")
    } else {
        ("", "", "", "")
    };

    for entry in &diff.entries {
        match entry {
            DiffEntry::Added(path) => {
                output.push(format!("{green}+ {path}{reset}"));
            }
            DiffEntry::Removed(path) => {
                output.push(format!("{red}- {path}{reset}"));
            }
            DiffEntry::MovedExplicit(path) => {
                // Use arrows for explicit moves
                output.push(format!("{cyan}↕ {path}{reset}"));
            }
            DiffEntry::MovedImplicit(path) => {
                output.push(format!("M {path}"));
            }
            DiffEntry::Unchanged(path) => {
                output.push(format!("U {path}"));
            }
        }
    }

    output.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_diff_no_changes() {
        let saved = "/a:/b:/c";
        let current = "/a:/b:/c";
        let affected = HashSet::new();
        let deleted = vec![];

        let diff = compute_diff(current, saved, &affected, &deleted, false);
        assert!(diff.is_empty());
    }

    #[test]
    fn test_compute_diff_addition() {
        let saved = "/a:/b";
        let current = "/a:/b:/c";
        let affected = HashSet::new();
        let deleted = vec![];

        let diff = compute_diff(current, saved, &affected, &deleted, false);
        assert_eq!(diff.entries.len(), 1);
        assert!(matches!(diff.entries[0], DiffEntry::Added(_)));
    }

    #[test]
    fn test_compute_diff_removal() {
        let saved = "/a:/b:/c";
        let current = "/a:/b";
        let affected = HashSet::new();
        let deleted = vec![];

        let diff = compute_diff(current, saved, &affected, &deleted, false);
        assert_eq!(diff.entries.len(), 1);
        assert!(matches!(diff.entries[0], DiffEntry::Removed(_)));
    }

    #[test]
    fn test_compute_diff_explicit_move() {
        let saved = "/a:/b:/c";
        let current = "/c:/a:/b";
        let mut affected = HashSet::new();
        affected.insert("/c".to_string());
        let deleted = vec![];

        let diff = compute_diff(current, saved, &affected, &deleted, false);
        // Only /c should show as explicitly moved
        assert!(diff
            .entries
            .iter()
            .any(|e| matches!(e, DiffEntry::MovedExplicit(p) if p == "/c")));
    }

    #[test]
    fn test_compute_diff_full_mode() {
        let saved = "/a:/b:/c";
        let current = "/c:/a:/b";
        let mut affected = HashSet::new();
        affected.insert("/c".to_string());
        let deleted = vec![];

        let diff = compute_diff(current, saved, &affected, &deleted, true);
        // Should show: /c explicitly moved, /a and /b implicitly moved
        assert!(diff
            .entries
            .iter()
            .any(|e| matches!(e, DiffEntry::MovedExplicit(_))));
        assert!(diff
            .entries
            .iter()
            .any(|e| matches!(e, DiffEntry::MovedImplicit(_))));
    }
}
