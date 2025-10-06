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

**Binary Entry Point:**
- **bin/whi.rs**: Main CLI entry point using Clap derive macros
  - `Cli` struct: Top-level argument parser with query args and subcommands
  - `Command` enum: All subcommands (both public and hidden `__*` variants)
  - `check_shell_integration()`: Guards public commands from running without shell integration
  - Dispatch logic routing commands to appropriate handlers in `app.rs`

**Core Application Logic:**
- **app.rs**: Business logic and command execution (1240 lines)
  - `run()`: Main execution flow for query operations
  - `search_name()`: Searches PATH for an executable by name
  - `handle_prefer()`: Implements the `--prefer` operation for making a specific executable "win"
  - `handle_*()`: Handler functions for move, swap, clean, delete operations
  - Session tracking, history management, and snapshot utilities

**CLI & Arguments:**
- **cli.rs**: Internal argument types and parsing helpers
  - `Args` struct: Internal representation of CLI options for business logic
  - `ColorWhen` enum: Color output modes
  - `PreferTarget`, `DeleteTarget` enums: Operation target types
  - `parse_prefer_arguments()`, `parse_delete_arguments()`: Helper parsers for hidden commands

**PATH Operations:**
- **path.rs**: PATH manipulation primitives (21K lines)
  - `PathSearcher`: Manages PATH directory list
  - `move_entry()`: Reorders PATH by moving an entry from one index to another
  - `swap_entries()`: Swaps two PATH entries
  - Uses 1-based indexing (user-facing) converted to 0-based internally

- **path_file.rs**: Whifile format parsing and serialization
  - `ParsedPathFile`: Represents parsed whifile with PATH and ENV sections
  - `format_path_file()`: Converts PATH string to whifile format
  - `parse_path_file()`: Parses whifile (supports both new `PATH!`/`ENV!` and legacy colon-separated formats)

- **path_resolver.rs**: Path resolution and fuzzy matching
  - `FuzzyMatcher`: Zoxide-style fuzzy path matching (used by delete, prefer operations)
  - `expand_tilde()`: Tilde expansion in paths
  - `resolve_path()`: Resolves relative/absolute paths with canonicalization
  - `looks_like_exact_path()`: Heuristic to distinguish paths from fuzzy patterns

- **path_diff.rs**: PATH diff utilities for showing changes

**File Operations:**
- **executor.rs**: File system operations
  - `ExecutableCheck`: Checks if a file exists and is executable
  - `SearchResult`: Contains path, canonical path, metadata, and PATH index
  - `FileMetadata`: inode, device, size, mtime, ctime

- **atomic_file.rs**: Atomic file write operations

**Output & Formatting:**
- **output.rs**: Output formatting
  - `OutputFormatter`: Handles colored output, indices, symlinks, metadata
  - Winner highlighting (green/bold), directory highlighting (yellow)
  - Time formatting without external dependencies

**Virtual Environments:**
- **venv_manager.rs**: Virtual environment management (620 lines)
  - `create_file()`: Creates whifile from current PATH
  - `source_from_path()`: Activates venv from whifile
  - `exit_venv()`: Deactivates current venv
  - `VenvTransition`: Represents state transitions (PATH changes, env var sets/unsets)
  - `expand_shell_vars()`: Expands `$VAR`, `${VAR}`, `$(cmd)`, backticks, and `~` in values

**Configuration & Profiles:**
- **config_manager.rs**: Profile management
  - `save_path()`: Saves current PATH as a profile
  - `load_profile()`: Loads saved PATH profile
  - `list_profiles()`, `remove_profile()`: Profile management
  - `ensure_whi_integration()`: Ensures whi integration exists in shell config

- **config.rs**: Configuration file utilities
  - Config directory management (`~/.whi/`)

**Session & History:**
- **history.rs**: Session history and undo/redo
  - `HistoryContext`: Manages session or venv-scoped history
  - Snapshot writing and navigation (undo/redo)

- **session_tracker.rs**: Session state tracking

**Shell Integration:**
- **shell_integration.rs**: Shell script generation
  - `generate_init_script()`: Outputs shell-specific initialization scripts

- **shell_detect.rs**: Shell detection and path utilities
  - Detects current shell (bash/zsh/fish)
  - Shell config file paths

**Embedded Shell Scripts (in src/):**
- **posix_integration.sh**: Bash/Zsh integration functions
- **fish_integration.fish**: Fish shell integration

**System Utilities:**
- **system.rs**: Low-level system operations
  - `get_parent_pid()`, `get_user_id()`: Process and user info
  - TTY detection using libc's `isatty(3)`

**Library Root:**
- **lib.rs**: Exports all public modules

### Key Design Decisions

1. **Minimal dependencies**: Only `libc` (for `isatty(3)`) and `clap` (for CLI parsing) - most functionality is implemented from scratch
2. **1-based indexing**: User-facing indices start at 1 (like human counting), converted to 0-based internally
3. **PATH indices matter**: All matches show their PATH index with `-i`, maximum 999 entries supported
4. **Winner concept**: First match in PATH order is the "winner" (what would actually execute)
5. **Shell integration**: PATH manipulation outputs new PATH string; shell functions apply it to current session
6. **Clap for CLI parsing**: Uses Clap 4.5 with derive macros for robust argument parsing while keeping dependency count minimal

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
- The Rust helper prints the transition lines. The shell function reads them and exports the new `PATH`, `VIRTUAL_ENV`, `VIRTUAL_ENV_PROMPT`, etc. The user's shell session now reflects the venv.
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

### Adding a new subcommand

1. **Define Clap structures in `bin/whi.rs`:**
   - Add public variant to `Command` enum (e.g., `Add(AddArgs)`)
   - Add hidden variant if shell integration needed (e.g., `HiddenAdd(HiddenAddArgs)`)
   - Define args struct using `#[derive(ClapArgs)]`

2. **Add dispatch logic in `bin/whi.rs` `main()`:**
   - Public command: call `check_shell_integration()` if mutation required
   - Hidden command: call handler directly (e.g., `run_hidden_add()`)

3. **Implement handler function:**
   - For app logic: add handler in `app.rs` (e.g., `handle_add_paths()`)
   - For plumbing: add runner in `bin/whi.rs` (e.g., `run_hidden_add()`)

4. **Update shell integration scripts:**
   - `posix_integration.sh`: Add case in `whi()` function
   - `fish_integration.fish`: Add case in `function whi`
   - Add shorthand function if desired (e.g., `whia`)

5. **Write tests:**
   - Unit tests in relevant module (`#[cfg(test)]`)
   - Integration tests in `tests/` directory (if applicable)

### Adding a new flag to existing command

1. Add field to appropriate Clap struct in `bin/whi.rs` with `#[arg(...)]` attribute
2. Pass flag through to `app.rs` handler (may need to update `cli::Args` struct)
3. Implement functionality in handler function
4. Update help text (automatically handled by Clap doc comments)

### Adding a new PATH operation

1. Add method to `PathSearcher` in `path.rs`
2. Add handler function in `app.rs` that uses the new method
3. Create Clap command variant and args struct in `bin/whi.rs`
4. Add dispatch case in `bin/whi.rs` `main()`
5. Update shell integration scripts if operation modifies PATH
6. Write tests in `path.rs` for the new method

### Output formatting changes

All output formatting goes through `OutputFormatter` in `output.rs`. Color codes are ANSI escape sequences applied directly when `use_color` is true.

### Parsing complex arguments for hidden commands

When shell integration passes complex arguments to hidden commands:

1. Define parser function in `cli.rs` (e.g., `parse_prefer_arguments()`)
2. Parse `Vec<String>` from Clap into semantic types (`PreferTarget`, `DeleteTarget`, etc.)
3. Call parser from hidden command handler in `bin/whi.rs`
4. Return `Result<T, String>` for error handling
