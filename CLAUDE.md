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

### Shell Integration Behaviour

Claude often misinterprets how `whi` works without the shell hooks, so keep these points in mind:

#### Why the integration is mandatory
- The Rust binary cannot mutate the parent shell environment on its own. Only the shell script (bash/zsh/fish) that invoked `whi` can export variables into the live session.
- The integration snippets export `WHI_SHELL_INITIALIZED=1` once they have finished wiring up helper functions and capturing the initial `PATH`. The binary treats this flag as proof that the integration is installed and it is safe to perform mutations.
- Every public verb that would change state calls `check_shell_integration()` first. If the flag is missing, the command prints integration instructions and exits with status `2`, intentionally making the verb a no-op.

#### Command categories
- **Guarded public commands:** Every entry point except `whi help` invokes `check_shell_integration()` immediately. That includes read-only queries (`whi <name>`, `--all`, `diff`, etc.) as well as mutation verbs. The goal is to give a single, consistent error message when the integration is missing.
- **Mutation verbs** (`prefer`, `move`, `switch`, `clean`, `delete`, `reset`, `undo`, `redo`, `load`, `list`, `save`, `apply`, `source`, `exit`, and variants that touch venv/session state) still rely on the integration to apply their effects, so they do nothing until `WHI_SHELL_INITIALIZED` is detected.
- **Hidden plumbing commands** (all `__*` subcommands like `__prefer`, `__move`, `__venv_source`, etc.) are invoked only by the shell integration. They assume the environment is trusted and therefore skip the guard.

#### Execution flow at runtime
1. You type `whi <verb>` in a configured shell.
2. The integration-defined shell function `whi()` intercepts the call. For verbs that change PATH or venv state it rewrites the invocation to the matching hidden subcommand.
3. The function runs the binary (`__whi_exec __<verb> ...`) and captures stdout/stderr.
4. Hidden commands return a transition description (tab-separated lines prefixed with `PATH`, `SET`, `UNSET`).
5. The shell function parses that transition and applies the exports/unsets inside the current shell session.

Because steps 2–5 never happen without the integration, the public verbs implement a belt-and-suspenders guard via `check_shell_integration()` so that direct CLI calls do not mislead users.

#### Mapping public verbs to hidden implementations

| User-facing verb | Hidden subcommand used by integration | Rust entry point | Result
| ---------------- | ------------------------------------- | ---------------- | ------ |
| `whi prefer` / `whip` | `__prefer` | `run_hidden_prefer()` | Reorder PATH so the named executable wins |
| `whi move` / `whim` | `__move` | `run_hidden_move()` | Move PATH entry between indices |
| `whi switch` / `whis` | `__switch` | `run_hidden_swap()` | Swap two PATH entries |
| `whi clean` / `whic` | `__clean` | `run_hidden_clean()` | Remove duplicate PATH entries |
| `whi delete` / `whid` | `__delete` | `run_hidden_delete()` | Delete PATH entries by index/path/pattern |
| `whi reset` | `__reset` | `run_hidden_reset()` | Restore PATH to session baseline |
| `whi undo` / `whiu` | `__undo` | `run_hidden_undo()` | Step backward through session history |
| `whi redo` / `whir` | `__redo` | `run_hidden_redo()` | Step forward through session history |
| `whi load` / `whil` | `__load` | `run_hidden_load()` | Load PATH from saved profile |
| `whi source` | `__venv_source` | `run_hidden_venv_source()` | Activate venv described by `whifile`/`whi.lock` |
| `whi exit` | `__venv_exit` | `run_hidden_venv_exit()` | Exit active venv and restore previous PATH |
| Prompt helpers, auto-activation checks | `__init`, `__should_auto_activate`, `__load_saved_path` | Dedicated `run_hidden_*` fns | Manage session bookkeeping |

Additional verbs such as `apply`, `save`, `list`, `remove-profile`, `file`, and `diff` run through their public code paths (no hidden wrapper), but they still invoke `check_shell_integration()` before doing any work. They need the integration because they rely on session state captured by the shell hooks (e.g., in-venv detection, saved baseline `PATH`, prompt markers) even though they do not emit tab-delimited transitions.

> The shell integration also provides aliases (`whip`, `whim`, etc.) that ultimately call the same hidden commands, so the table above covers both spellings.

#### Concrete example: `whi source`
- `whi source` in a configured shell triggers the `whi()` shell function. After basic validation it calls `__whi_venv_source "$PWD"`.
- `__whi_venv_source` executes `whi __venv_source <path>` which enters `Command::HiddenVenvSource` and calls `run_hidden_venv_source()`.
- `run_hidden_venv_source()` delegates to `whi::venv_manager::source_from_path()`, producing a `VenvTransition` object.
- The Rust helper prints the transition lines. The shell function reads them and exports the new `PATH`, `WHI_VENV_*` markers, etc. The user’s shell session now reflects the venv.
- If the integration were missing, the public `whi source` arm would stop at `check_shell_integration()` and exit with code `2`, so no misleading “success” occurs.

The same pattern applies to `whi exit` and every other mutating verb: public variants exist largely for UX/help output, while the hidden variants (plus the tab-delimited transition protocol) do the actual environment mutation once the integration is active.

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

---

## MCP Usage Guide

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

**Purpose:** Read-only AST queries and metrics (sub-second, no indexing). Cannot edit code.
**Use for:** Finding symbols/patterns, complexity hot spots, dependency/import maps.
**Avoid for:** Editing code (use Serena); fetching docs (use Context7).

**Session setup (every new session)**

```python
register_project_tool(path="/abs/path/to/repo", name="repo")  # required each session
```

**Core calls**

```python
# Analysis
analyze_project("repo")                     # files, languages, build files
analyze_complexity("repo", "src/app.rs")    # cyclomatic, avg lines, ratios
get_symbols("repo", "src/file.rs")          # functions, structs, imports (deep)
get_dependencies("repo", "src/file.rs")     # clean import list

# Pattern matching
list_query_templates_tool("rust")           # built-in patterns (functions, structs, enums...)
build_query("rust", ["functions", "structs"], combine="or")
run_query("repo", "(function_item name: (identifier) @name)", language="rust")

# Deep inspection
get_ast("repo", "src/file.rs", max_depth=3) # full AST tree
find_usage("repo", "SymbolName", file_path="src/file.rs")
```

**Tips**

* Use query templates to avoid writing raw S-expressions.
* Pair with Serena: Tree-sitter finds what to change, Serena makes the change.

---

### Serena — Symbol-Based Editing

**Purpose:** Edit by **symbol name** (10× cheaper: 200 tokens vs 2,350 for Read+Edit).
**Use for:** Replacing function/method **bodies**, inserting imports/attrs, adding methods.
**Avoid for:** Finding symbols (use Tree-sitter first), changing function **signatures**.

**Core calls (body-only!)**

```python
# 10x more efficient than Read + Edit
replace_symbol_body("TypeOrMod/method_name", "src/file.rs", """    // new body""")
insert_before_symbol("TypeOrMod/Target", "src/file.rs", "use crate::thing::Item;")
insert_after_symbol("TypeOrMod/Target", "src/file.rs", """
pub fn helper(&self) { /* ... */ }
""")
```

**Tips**

* Use full name paths (e.g., `StructName/method_name`) to avoid collisions.
* `replace_symbol_body` only replaces the block **inside `{}`**.
* Robust: edits by symbol name, not line numbers (survives file changes).

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

**Core distinction**

* **Tree-sitter** = Read-only analysis (find what to change)
* **Serena** = Symbol-based editing (make the change)
* They're complementary, not competing

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
