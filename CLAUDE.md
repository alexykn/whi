# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`whi` is a command-line utility written in Rust that serves as a powerful replacement for the Unix `which` command with PATH manipulation capabilities. It finds executables in PATH, shows all matches with indices, and provides shell integration for dynamically reordering PATH entries.

## Building and Testing

```bash
# Build the project
cargo build --release

# Run tests
cargo test

# Run specific test
cargo test <test_name>

# Install locally
cargo install --path .
```

The compiled binary will be at `./target/release/whi`.

## Architecture

### Module Structure

- **main.rs**: Entry point, argument parsing dispatch, and core logic orchestration
  - `run()`: Main execution flow handling all operations (search, move, swap, prefer)
  - `get_names()`: Reads executable names from args or stdin
  - `search_name()`: Searches PATH for an executable by name
  - `handle_prefer()`: Implements the `--prefer` operation for making a specific executable "win"
  - `atty` module: TTY detection using libc's `isatty(3)`

- **cli.rs**: Command-line argument parsing
  - `Args` struct: All command-line options and flags
  - Manual argument parsing (no external dependencies)
  - Support for combined Unix-style flags (e.g., `-ail` = `-a -i -l`)

- **path.rs**: PATH manipulation logic
  - `PathSearcher`: Manages PATH directory list
  - `move_entry()`: Reorders PATH by moving an entry from one index to another
  - `swap_entries()`: Swaps two PATH entries
  - Uses 1-based indexing (user-facing) converted to 0-based internally

- **executor.rs**: File system operations
  - `ExecutableCheck`: Checks if a file exists and is executable
  - `SearchResult`: Contains path, canonical path, metadata, and PATH index
  - `FileMetadata`: inode, device, size, mtime, ctime

- **output.rs**: Output formatting
  - `OutputFormatter`: Handles colored output, indices, symlinks, metadata
  - Winner highlighting (green/bold), directory highlighting (yellow)
  - Time formatting without external dependencies

- **shell_integration.rs**: Shell function generation
  - `generate_init_script()`: Outputs shell-specific functions for bash/zsh/fish
  - Provides `whim`, `whis`, `whip`, `whia`, `whii` convenience commands
  - Shell functions modify PATH in the current shell session

### Key Design Decisions

1. **Zero dependencies**: Only `libc` for `isatty(3)` - all other functionality is implemented from scratch
2. **1-based indexing**: User-facing indices start at 1 (like human counting), converted to 0-based internally
3. **PATH indices matter**: All matches show their PATH index with `-i`, maximum 999 entries supported
4. **Winner concept**: First match in PATH order is the "winner" (what would actually execute)
5. **Shell integration**: PATH manipulation outputs new PATH string; shell functions apply it to current session
6. **Manual argument parsing**: No clap/structopt to minimize dependencies

### PATH Manipulation Flow

The `--move`, `--swap`, and `--prefer` operations follow this pattern:
1. Parse PATH into vector of PathBuf entries
2. Perform operation (move/swap) on the vector
3. Convert back to colon-separated PATH string
4. Output to stdout (shell functions capture and apply to $PATH)

The `--prefer NAME INDEX` operation:
1. Searches PATH for all occurrences of executable NAME
2. Identifies current winner (lowest index)
3. If target is at higher index, moves it to just before winner
4. Returns error if target is already winning

## Testing

Tests are embedded in source files using `#[cfg(test)]`:
- **path.rs**: Comprehensive tests for `move_entry()` and `swap_entries()`
  - Forward/backward moves, first/last positions, same position
  - Boundary conditions, zero indices, out of bounds

No integration tests directory exists yet. When adding integration tests, follow Rust conventions and place them in `tests/` directory.

## Common Patterns

### Adding a new flag

1. Add field to `Args` struct in cli.rs
2. Add parsing logic in `process_arg()` and/or `parse_combined_flags()`
3. Update `print_help()` with new flag documentation
4. Implement functionality in main.rs `run()` function

### Adding a new PATH operation

1. Add method to `PathSearcher` in path.rs
2. Add corresponding field to `Args` struct
3. Handle operation in main.rs `run()` function
4. Add shell integration function in shell_integration.rs (if needed)
5. Write tests in path.rs

### Output formatting changes

All output formatting goes through `OutputFormatter` in output.rs. Color codes are ANSI escape sequences applied directly when `use_color` is true.

Here’s a concise, drop-in CLAUDE.md section you can paste into any project.

---

## MCP: Usage Guide

This section explains how to use the four MCP tools—**Context7**, **Tree-sitter**, **Serena**, and **Git (Local)**—and when to choose each one.

---

### Context7 — Library Docs & Examples

**Purpose:** Instant API lookups and code snippets (sub-second).
**Use for:** “How do I …?” syntax, validators/routing/DI patterns, quick examples.
**Avoid for:** Deep “why” questions, discovery/research, niche libs with low coverage.

**Core calls**

```python
resolve-library-id("fastapi")  # choose highest trust + most snippets
get-library-docs("/websites/fastapi_tiangolo", topic="dependency injection", tokens=2000)
# topic is optional; tokens default ~5000 (1k–20k)
```

**Tips**

* Prefer libraries with Trust ≥ 7.0 and high snippet counts.
* Use `topic` to narrow results; source URLs are included for deeper reading.

---

### Tree-sitter — AST Analysis (Read-only)

**Purpose:** Cross-file, language-agnostic AST queries and metrics (sub-second, no indexing).
**Use for:** Finding symbols/patterns, complexity hot spots, dependency/import maps.
**Avoid for:** Editing code; fetching docs (use Context7).

**Session setup (every new session)**

```python
register_project_tool(path="/abs/path/to/repo", name="repo")  # required each session
```

**Core calls**

```python
analyze_project("repo")                     # files, languages, build files
analyze_complexity("repo", "src/app.rs")    # cyclomatic, avg lines, ratios
get_symbols("repo", "src/file.rs")          # functions, structs, imports (deep)
get_dependencies("repo", "src/file.rs")     # clean import list
list_query_templates_tool("rust")           # built-in patterns
run_query("repo", "(function_item name: (identifier) @name)", language="rust")
```

**Tips**

* Use query templates to avoid writing raw S-expressions.
* Great for “find then edit” workflows (pair with Serena).

---

### Serena — Symbol-Based Editing

**Purpose:** Token-efficient edits by **symbol**, not line numbers (~10× cheaper than full-file edits).
**Use for:** Replacing function/method **bodies**, inserting imports/attrs, adding methods.
**Avoid for:** Finding symbols (use Tree-sitter first), changing function **signatures**.

**Core calls (body-only!)**

```python
replace_symbol_body("TypeOrMod/method_name", "src/file.rs", """    // new body""")
insert_before_symbol("TypeOrMod/Target", "src/file.rs", "use crate::thing::Item;")
insert_after_symbol("TypeOrMod/Target", "src/file.rs", """
pub fn helper(&self) { /* ... */ }
""")
```

**Tips**

* Use full name paths (e.g., `StructName/method_name`) to avoid collisions.
* `replace_symbol_body` only replaces the block **inside `{}`**.

---

### Git (Local) — Safe Review → Stage → Commit

**Purpose:** Structured, safe local Git operations.
**Use for:** Status/diffs, staging, atomic commits, quick history/branch ops.
**Avoid for:** Push/pull/fetch/rebase/stash/force (run in shell manually).

**Core calls**

```python
git_status("/abs/path/to/repo")
git_diff_unstaged("/abs/path/to/repo", context_lines=3)
git_add("/abs/path/to/repo", ["src/file.rs"])
git_diff_staged("/abs/path/to/repo")
git_commit("/abs/path/to/repo", "feat: concise commit message")

git_log("/abs/path/to/repo", max_count=5)
git_show("/abs/path/to/repo", "HEAD~1")
git_branch("/abs/path/to/repo", "local")
git_create_branch("/abs/path/to/repo", "feature/x", base_branch="main")
git_checkout("/abs/path/to/repo", "feature/x")
git_diff("/abs/path/to/repo", "HEAD~2..HEAD")
```

**Tips**

* Always review unstaged and staged diffs before committing.
* One logical change per commit (atomic commits).

---

### Which Tool When? (Quick Guide)

**Primary decision rule**

* **Need docs/examples fast?** → **Context7**
* **Need to *find* code patterns/symbols?** → **Tree-sitter**
* **Need to *edit* code by symbol efficiently?** → **Serena**
* **Need to review/stage/commit safely?** → **Git (Local)**

**Common tasks → tool**

| Task                                                                    | Tool                                                                           |
| ----------------------------------------------------------------------- | ------------------------------------------------------------------------------ |
| “How do I write a field validator in Pydantic?”                         | **Context7**                                                                   |
| “List all public functions / find all structs / imports across project” | **Tree-sitter**                                                                |
| “Replace body of `run()` / insert import / add helper method”           | **Serena**                                                                     |
| “Show changes → stage selected files → commit”                          | **Git (Local)**                                                                |
| “Find all uses of deprecated API pattern”                               | **Tree-sitter** (then **Serena** to fix)                                       |
| “Add feature using new library I don’t know”                            | **Context7**, then **Serena**, verify with **Tree-sitter**, commit via **Git** |

**Canonical workflows**

1. **Single-symbol refactor:** Tree-sitter (find) → Serena (edit) → Git (review/commit)
2. **Cross-file pattern fix:** Tree-sitter (query across repo) → Serena (apply to matches) → Git
3. **Learn & implement:** Context7 (examples) → Serena (code) → Tree-sitter (sanity check) → Git

**Session init (minimal)**

```python
register_project_tool("/abs/path/to/repo", "repo")
git_status("/abs/path/to/repo")
````
