# whi

**Magically simple PATH management**

`whi` is a powerful `which` replacement with PATH manipulation. Find executables, see all matches, and reorder your PATH with simple shell commands.

## Features

- **Better than which**: Shows winner by default, all matches with `-a`
- **PATH manipulation**: Move, swap, and prefer executables with shell integration
- **Winner indication**: Clearly marks which executable would actually run with color
- **PATH indices** (`-i`): Shows the PATH index for each match
- **Full PATH listing** (`-f`): Displays complete PATH with indices
- **Follow symlinks** (`-l`): Resolves and shows canonical targets
- **File metadata** (`-s`): Shows inode, device, size, and modification time
- **Combinable flags**: Unix-style flag combining (e.g., `-ais`, `-ifl`)
- **Pipe-friendly**: Quiet by default, all output to stdout
- **Zero dependencies**: Only libc for isatty(3)

## Installation

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

The `whi init <shell>` command outputs shell-specific functions that you can evaluate/source. This provides four powerful commands:

- **`whim FROM TO`** - Move PATH entry from index FROM to index TO
  ```bash
  $ whim 10 1      # Move entry at index 10 to position 1
  ```

- **`whis IDX1 IDX2`** - Swap two PATH entries
  ```bash
  $ whis 10 41     # Swap entries at indices 10 and 41
  ```

- **`whip NAME INDEX`** - Make executable at INDEX win (prefer it over others)
  ```bash
  $ whip cargo 50  # Make cargo at index 50 the winner
  ```

- **`whia NAME`** - Show all matches with indices (shortcut for `whi -ia`)
  ```bash
  $ whia cargo     # Equivalent to: whi -ia cargo
  ```

These commands actually modify your current shell's PATH environment variable, so changes take effect immediately without restarting your shell or sourcing config files

### Basic Usage

Find an executable (like `which`):

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

With PATH indices (or use `whia cargo` shortcut):

```bash
$ whi -ai cargo
[40] /Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
[41] /opt/homebrew/bin/cargo
[50] /Users/user/.cargo/bin/cargo
```

### PATH Manipulation

Make a different version win:

```bash
$ whip cargo 50  # Move cargo at index 50 to win
```

Move PATH entries around:

```bash
$ whim 10 1      # Move entry 10 to position 1
```

Swap two PATH entries:

```bash
$ whis 10 41     # Swap entries at 10 and 41
```

Read names from stdin:

```bash
$ echo -e "python\\nnode\\ncargo" | whi
```

Follow symlinks (shows original → canonical):

```bash
$ whi -aL cargo
/opt/homebrew/bin/cargo → /opt/homebrew/Cellar/rustup/1.28.2/bin/rustup-init
/Users/user/.cargo/bin/cargo → /Users/user/.cargo/bin/rustup
/Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
```

Override PATH:

```bash
$ whi --path="/usr/local/bin:/usr/bin" python
```

## Command-Line Options

### Flags

Short flags can be combined Unix-style (e.g., `-ai` = `-a -i`, `-ais` = `-a -i -s`).

- **`-a, --all`** - Show all PATH matches (default: only winner)
- **`-f, --full`** - Show full PATH directory listing
- **`-i, --index`** - Show PATH index next to each entry
- **`-l, -L, --follow-symlinks`** - Resolve and show canonical targets
- **`-o, --one`** - Only print the first match per name
- **`-s, --stat`** - Include inode/device/mtime/size metadata
- **`-0, --print0`** - NUL-separated output for use with xargs
- **`-q, --quiet`** - Suppress non-fatal stderr warnings
- **`--silent`** - Print nothing to stderr, use exit codes only
- **`--show-nonexec`** - Also list files that exist but aren't executable
- **`-h, --help`** - Print help information

### PATH Manipulation

These commands output a modified PATH string to stdout. Use shell integration (see above) to actually modify your current shell's PATH.

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

- **`--prefer <NAME> <INDEX>`** - Make executable NAME at INDEX win
  ```bash
  $ whi --prefer cargo 50
  /modified/path/string/...
  ```

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

Check which node would run with indices:

```bash
$ whi -i node
```

Use with xargs to check executables:

```bash
$ whi -0 python node cargo | xargs -0 -n1 file
```

Find all versions with metadata:

```bash
$ whi -ais gcc
```

Show full PATH listing with indices:

```bash
$ whi -f
[1] /usr/local/bin
[2] /usr/bin
[3] /bin
...
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

`whi` answers all these questions and lets you manipulate your PATH on the fly with simple shell commands.

## License

MIT

## Author

Alexander Knott <alexander.knott@posteo.de>