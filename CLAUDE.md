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