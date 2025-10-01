# whi

**Stupid simple PATH management**

`whi` is a powerful `which` replacement with PATH manipulation. Find executables, see all matches, and reorder your PATH with simple shell commands.

## Key Feature: Session-Based with Optional Persistence

**By default, `whi` only modifies your current shell session.** Changes are temporary and safe to experiment with. When you're happy with your PATH, use `whi diff` to review changes and `whi save` to persist them.

```bash
# 1. Manipulate PATH in current session (temporary)
$ whid 5 16 7    # Delete entries at indices 5, 16, and 7
$ whic           # Clean duplicate entries
$ whim 10 1      # Move entry at index 10 to position 1

# 2. Review changes before saving
$ whi diff
+ /usr/local/bin/new-tool
- /old/removed/path
↕ /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin

# 3. Persist changes across new terminal sessions
$ whi save
Saved PATH to zsh (68 entries)

# Open new terminal → your changes are still there!
```

**This workflow prevents accidental PATH corruption** and lets you experiment freely.

## Features

- **Session-based by default**: Changes only affect current shell, safe to experiment
- **Persistence when you want it**: Use `whi diff` to review, `whi save` to persist
- **Better than which**: Shows winner by default, all matches with `-a`
- **PATH manipulation**: Move, swap, delete, clean duplicates, and prefer executables
- **Path-based operations**: Add/prefer paths directly, supports tilde expansion and relative paths
- **Fuzzy matching**: Zoxide-style pattern matching for paths (no quotes needed)
- **Winner indication**: Clearly marks which executable would actually run with color
- **PATH indices** (`-i`): Shows the PATH index for each match
- **Full PATH listing** (`-f`): Displays complete PATH with indices
- **Follow symlinks** (`-l`): Resolves and shows canonical targets
- **File metadata** (`-s`): Shows inode, device, size, and modification time
- **Combinable flags**: Unix-style flag combining (e.g., `-ais`, `-ifl`)
- **Pipe-friendly**: Quiet by default, all output to stdout
- **Zero dependencies**: Only libc for isatty(3)

## Installation

From crates.io:

```bash
cargo install whi
```

Or build from source:

```bash
git clone https://github.com/alexykn/whi
cd whi
cargo build --release
```

## Quick Start

### Shell Integration (Recommended)

`whi` provides shell integration that gives you commands to manipulate your PATH directly in your current shell session. Without shell integration, `whi --move` and similar commands would only output a new PATH string - they can't modify your actual shell's PATH variable.

To enable shell integration, add this to your shell config:

**Bash** (`~/.bashrc`):
```bash
eval "$(whi init bash)"
```

**Zsh** (`~/.zshrc`):
```bash
eval "$(whi init zsh)"
```

**Fish** (`~/.config/fish/config.fish`):
```fish
whi init fish | source
```

The `whi init <shell>` command outputs shell-specific functions that you can evaluate/source. This provides seven powerful commands:

- **`whim FROM TO`** - Move PATH entry from index FROM to index TO
  ```bash
  $ whim 10 1      # Move entry at index 10 to position 1
  ```

- **`whis IDX1 IDX2`** - Swap two PATH entries
  ```bash
  $ whis 10 41     # Swap entries at indices 10 and 41
  ```

- **`whip NAME TARGET`** - Make executable win, or add path to PATH
  ```bash
  # Index-based (original)
  $ whip cargo 50                  # Make cargo at index 50 the winner

  # Path-based (new)
  $ whip cargo ~/.cargo/bin        # Add/prefer cargo from ~/.cargo/bin
  $ whip bat ./target/release      # Prefer bat from relative path

  # Fuzzy matching (new, no quotes needed)
  $ whip bat github release        # Prefer bat from path matching 'github' and 'release'

  # Path-only (new, like fish_add_path)
  $ whip ~/.local/bin              # Add path to PATH if not present
  ```

- **`whic`** - Clean duplicate PATH entries (keeps first occurrence)
  ```bash
  $ whic           # Remove all duplicate entries
  ```

- **`whid TARGET...`** - Delete PATH entries by index, path, or fuzzy pattern
  ```bash
  # Index-based (original)
  $ whid 5                       # Delete entry at index 5
  $ whid 5 16 7                  # Delete entries at indices 5, 16, and 7

  # Path-based (new)
  $ whid /opt/homebrew/bin       # Delete exact path
  $ whid ~/.local/bin            # Delete with tilde expansion

  # Fuzzy matching (new, deletes ALL matches)
  $ whid old temp                # Delete ALL paths matching 'old' and 'temp'
  $ whid local bin               # Delete ALL paths matching pattern
  ```

- **`whia NAME`** - Show all matches with indices (shortcut for `whi -a`)
  ```bash
  $ whia cargo     # Equivalent to: whi -a cargo
  ```

**Important:** These commands only modify your **current shell session**. Changes are temporary until you use `whi save` to persist them.

### Basic Usage

View all PATH entries (indices shown by default):

```bash
$ whi
[1] /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin
[2] /opt/homebrew/bin
[3] /usr/local/bin
[4] /usr/bin
...

$ whi -n    # Hide indices with --no-index
/Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin
/opt/homebrew/bin
/usr/local/bin
/usr/bin
...
```

Find an executable (like `which`) - shows only the winner:

```bash
$ whi cargo
/Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
```

See all matches with `-a`:

```bash
$ whi -a cargo
/Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
/opt/homebrew/bin/cargo
/Users/user/.cargo/bin/cargo
```

With PATH indices using `-i` (or use `whia cargo` shortcut):

```bash
$ whi -ai cargo
[1] /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
[2] /opt/homebrew/bin/cargo
[5] /Users/user/.cargo/bin/cargo
```

Follow symlinks with `-l` to see what they point to:

```bash
$ whi -ail cargo
[1] /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
[2] /opt/homebrew/bin/cargo → /opt/homebrew/Cellar/rustup/1.28.2/bin/rustup-init
[5] /Users/user/.cargo/bin/cargo → /Users/user/.cargo/bin/rustup
```

Show detailed metadata with `-s`:

```bash
$ whi -as cargo
/Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
  inode: 152583153, device: 16777233, size: 30854216 bytes
  created:  2025-09-27 10:30:17
  modified: 2025-09-27 10:30:17
/opt/homebrew/bin/cargo
  inode: 103789726, device: 16777233, size: 11154288 bytes
  created:  2025-04-28 15:56:34
  modified: 2025-04-28 15:56:34
/Users/user/.cargo/bin/cargo
  inode: 117539552, device: 16777233, size: 11174016 bytes
  created:  2025-09-03 07:12:56
  modified: 2025-09-03 07:12:56
```

Combine flags for all matches with indices and symlinks:

```bash
$ whi -ail python
[3] /usr/local/bin/python → /usr/local/Cellar/python@3.11/3.11.5/bin/python3.11
[8] /usr/bin/python
[12] /opt/homebrew/bin/python → /opt/homebrew/Cellar/python@3.12/3.12.0/bin/python3.12
```

### PATH Manipulation Examples

See which cargo is winning and make a different one win:

```bash
$ whia cargo
[1] /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
[2] /opt/homebrew/bin/cargo
[5] /Users/user/.cargo/bin/cargo

$ whip cargo 5    # Make cargo at index 5 the winner
$ whia cargo
[1] /Users/user/.cargo/bin/cargo
[2] /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
[3] /opt/homebrew/bin/cargo
```

Move PATH entries to reorder them:

```bash
$ whim 10 1      # Move entry at index 10 to position 1
$ whim 50 3      # Move entry at index 50 to position 3
```

Swap two PATH entries:

```bash
$ whis 10 41     # Swap entries at indices 10 and 41
```

Clean duplicate entries:

```bash
$ whia cargo
[4] /Users/user/.cargo/bin/cargo
[6] /opt/homebrew/bin/cargo
[54] /Users/user/.cargo/bin/cargo    # Duplicate!

$ whic           # Remove duplicates
$ whia cargo
[4] /Users/user/.cargo/bin/cargo
[6] /opt/homebrew/bin/cargo
```

Delete specific entries:

```bash
$ whid 6 54      # Delete entries at indices 6 and 54
$ whid 5 16 7    # Delete multiple entries at once
```

### Making Changes Persistent

**All the commands above only affect your current shell session.** To make changes permanent:

```bash
# 1. Make changes in your current session
$ whic                           # Clean duplicates
$ whid 10 20                     # Delete unwanted entries
$ whim 5 1                       # Reorder as needed

# 2. Review what changed
$ whi diff
- /Users/user/.rye/shims
- /Users/user/.cargo/bin
↕ /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin

# 3. Save changes (persists across new terminal sessions)
$ whi save
Saved PATH to zsh (65 entries)

# Or save to all shells at once
$ whi save all
Saved PATH to bash (65 entries)
Saved PATH to zsh (65 entries)
Saved PATH to fish (65 entries)
```

After running `whi save`, your changes are automatically loaded in new terminal sessions. The saved PATH is stored in `~/.whi/saved_path_<shell>` and loaded by a single line added to your shell config file.

**You can experiment safely** because changes are session-only until you explicitly save them.

### Understanding `whi diff`

The `whi diff` command shows what changed between your current session and the saved PATH. It uses intelligent markers to distinguish different types of changes:

**Basic diff** (`whi diff`) - Shows only explicit changes:
```bash
$ whi diff
+ /new/path/added                    # You added this path
- /old/path/removed                  # Deleted with whid
- /Users/user/.cargo/bin             # Duplicate removed by whic
↕ /Users/user/.rustup/.../bin        # You moved this with whim/whis/whip
```

**Full diff** (`whi diff full`) - Shows everything including implicit shifts:
```bash
$ whi diff full
+ /new/path/added                    # Explicitly added
- /old/path/removed                  # Explicitly deleted
↕ /Users/user/.rustup/.../bin        # Explicitly moved by you
M /opt/homebrew/bin                  # Implicitly shifted (side effect)
M /usr/local/bin                     # Implicitly shifted (side effect)
U /usr/bin                           # Unchanged position
U /bin                               # Unchanged position
```

**Diff markers explained:**
- **`+`** (green) - New path added to your PATH
- **`-`** (red) - Path removed (via `whid` or duplicate removed by `whic`)
- **`↕`** (cyan) - Path explicitly moved by you using `whim`, `whis`, or `whip`
- **`M`** - Path implicitly moved (shifted as a side effect of your operations) - *full mode only*
- **`U`** - Path unchanged (same position as saved) - *full mode only*

**Session tracking:** `whi` tracks all operations in your current shell session (`whim`, `whis`, `whip`, `whic`, `whid`) to accurately distinguish between changes you explicitly made versus paths that shifted as a side effect. This makes `whi diff` highly accurate in showing what you actually changed.

**Use cases:**
```bash
# Quick check - see only what you explicitly changed
$ whi diff

# Detailed review - see everything including ripple effects
$ whi diff full

# Compare with saved PATH from a specific shell
$ whi diff bash
$ whi diff zsh
$ whi diff fish
```

### Other Usage Examples

Read multiple names from stdin:

```bash
$ echo -e "python\\nnode\\ncargo" | whi
/usr/bin/python
/usr/local/bin/node
/Users/user/.cargo/bin/cargo
```

Check multiple executables at once:

```bash
$ whi python node cargo gcc
/usr/bin/python
/usr/local/bin/node
/Users/user/.cargo/bin/cargo
/usr/bin/gcc
```

Show all PATH entries (indices shown by default):

```bash
$ whi          # With indices (default)
[1] /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin
[2] /opt/homebrew/bin
[3] /usr/local/bin
[4] /usr/bin
...

$ whi -n       # Plain listing (no indices)
/Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin
/opt/homebrew/bin
/usr/local/bin
/usr/bin
...
```

Use custom PATH:

```bash
$ whi --path="/usr/local/bin:/usr/bin" python
/usr/local/bin/python
```

## Command-Line Options

### Flags

Short flags can be combined Unix-style (e.g., `-al` = `-a -l`, `-als` = `-a -l -s`).

- **`-a, --all`** - Show all PATH matches (default: only winner)
- **`-f, --full`** - Show all matches + full PATH listing (implies `-a`; directories with matches highlighted in color)
- **`-n, --no-index`** - Hide PATH index (shown by default)
- **`-l, -L, --follow-symlinks`** - Resolve and show canonical targets
- **`-o, --one`** - Only print the first match per name
- **`-s, --stat`** - Include inode/device/mtime/size metadata
- **`-0, --print0`** - NUL-separated output for use with xargs
- **`-q, --quiet`** - Suppress non-fatal stderr warnings
- **`--silent`** - Print nothing to stderr, use exit codes only
- **`--show-nonexec`** - Also list files that exist but aren't executable
- **`-h, --help`** - Print help information

### PATH Manipulation (Session Only)

These commands output a modified PATH string to stdout. Use shell integration (see above) to actually modify your current shell's PATH. **Changes are temporary** until you use `whi save`.

- **`--move <FROM> <TO>`** - Move PATH entry from index FROM to index TO
  ```bash
  $ whi --move 10 1
  /path/at/10:/path/at/1:/path/at/2:...
  ```

- **`--swap <IDX1> <IDX2>`** - Swap PATH entries at indices IDX1 and IDX2
  ```bash
  $ whi --swap 10 41
  /modified/path/string/...
  ```

- **`--prefer <NAME> <TARGET>`** or **`--prefer <PATH>`** - Make executable win or add path
  ```bash
  # Index-based
  $ whi --prefer cargo 50
  /modified/path/string/...

  # Path-based (adds path at winning position if not present)
  $ whi --prefer cargo ~/.cargo/bin
  $ whi --prefer bat /usr/local/bin

  # Fuzzy matching (must match exactly one path)
  $ whi --prefer bat github release

  # Path-only (like fish_add_path)
  $ whi --prefer ~/.local/bin
  ```

- **`--clean` / `-c`** - Remove duplicate PATH entries (keeps first occurrence)
  ```bash
  $ whi --clean
  /deduplicated/path/...
  ```

- **`--delete <TARGET>...` / `-d`** - Delete PATH entries by index, path, or fuzzy pattern
  ```bash
  # Index-based
  $ whi --delete 5 16 7
  /path/without/those/entries...

  # Path-based
  $ whi --delete /opt/homebrew/bin
  $ whi --delete ~/.local/bin

  # Fuzzy matching (deletes ALL matches)
  $ whi --delete old temp
  $ whi --delete local bin
  ```

### Persistence

- **`whi save [SHELL]`** - Save current PATH persistently
  ```bash
  $ whi save           # Auto-detect current shell
  $ whi save bash      # Save for bash
  $ whi save zsh       # Save for zsh
  $ whi save fish      # Save for fish
  $ whi save all       # Save for all shells
  ```

- **`whi diff [SHELL|full]`** - Show differences between current and saved PATH
  ```bash
  $ whi diff           # Compare with saved PATH for current shell (explicit changes only)
  $ whi diff full      # Show everything including implicit shifts (M) and unchanged (U)
  $ whi diff zsh       # Compare with saved PATH for zsh
  $ whi diff fish      # Compare with saved PATH for fish
  ```

  **Diff markers:**
  - `+` (green) - Added paths
  - `-` (red) - Removed paths (including duplicates from `whic`)
  - `↕` (cyan) - Explicitly moved paths (`whim`/`whis`/`whip`)
  - `M` - Implicitly shifted paths (*full mode only*)
  - `U` - Unchanged paths (*full mode only*)

After `whi save`, changes persist across new terminal sessions. Use `whi diff` to review changes before saving.

### Other Options

- **`--path <PATH>`** - Override environment PATH string
  ```bash
  $ whi --path="/usr/local/bin:/usr/bin" python
  ```

- **`--color <WHEN>`** - Colorize output: `auto`, `never`, `always` [default: auto]
  ```bash
  $ whi --color=always cargo
  ```

### Shell Integration Command

- **`whi init <SHELL>`** - Output shell integration code for bash, zsh, or fish
  ```bash
  $ whi init bash    # Output bash functions
  $ whi init zsh     # Output zsh functions
  $ whi init fish    # Output fish functions
  ```

## Exit Codes

- `0` - All names found
- `1` - At least one not found
- `2` - Usage error
- `3` - I/O or environment error

## Examples

Find all versions of Python in PATH:

```bash
$ whi -a python python3 python3.11
```

Check which node would run:

```bash
$ whi node
```

Use with xargs to check executables:

```bash
$ whi -0 python node cargo | xargs -0 -n1 file
```

Find all versions with metadata:

```bash
$ whi -ais gcc
```

Show all PATH entries with indices:

```bash
$ whi          # Indices shown by default
[1] /usr/local/bin
[2] /usr/bin
[3] /bin
...
```

Prefer cargo from a specific path (adds if needed):

```bash
$ whip cargo ~/.cargo/bin
```

Add a path to PATH (like fish_add_path):

```bash
$ whip ~/.local/bin
```

Use fuzzy matching to prefer bat from GitHub release:

```bash
$ whip bat github release    # Matches /path/to/github/whi/target/release
```

Delete all temporary build paths:

```bash
$ whid temp build            # Deletes ALL paths matching both 'temp' and 'build'
```

## Comparison with `which`

| Feature | `which` | `whi` |
|---------|---------|----------|
| Show first match | ✓ | ✓ (default) |
| Show all matches | Some versions with `-a` | ✓ With `-a` |
| Show PATH indices | ✗ | ✓ With `-i` |
| Full PATH listing | ✗ | ✓ With `-f` |
| Follow symlinks | Some versions | ✓ With `-l/-L` |
| File metadata | ✗ | ✓ With `-s` |
| PATH manipulation | ✗ | ✓ With shell integration |
| Clean duplicates | ✗ | ✓ With `whic` |
| Delete entries | ✗ | ✓ With `whid` |
| Session-based changes | ✗ | ✓ Safe to experiment |
| Persistent changes | ✗ | ✓ With `whi save` |
| Intelligent diff | ✗ | ✓ With `whi diff` (tracks explicit vs implicit changes) |
| Combinable flags | ✗ | ✓ |
| Multiple names | ✓ | ✓ |
| Stdin input | ✗ | ✓ |
| Pipe-friendly | Varies | ✓ |

## Why?

Ever wonder:
- Which version of `python` or `node` is actually running?
- How to make a different version win without editing shell configs?
- What other versions exist on your PATH?
- What's the actual order of your PATH directories?
- How to safely experiment with PATH changes without breaking things?

`whi` answers all these questions and lets you:
- **Experiment safely** with session-only changes
- **Review before committing** with `whi diff`
- **Persist when ready** with `whi save`
- **Manipulate PATH on the fly** without manually editing config files

## License

MIT

## Author

Alexander Knott <alexander.knott@posteo.de>
