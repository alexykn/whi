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

Add to your shell config (e.g., `~/.bashrc`, `~/.zshrc`, or `~/.config/fish/config.fish`):

```bash
# bash/zsh
eval "$(whi init bash)"  # or zsh

# fish
whi init fish | source
```

This provides four powerful commands:
- `whim FROM TO` - Move PATH entry
- `whis IDX1 IDX2` - Swap PATH entries
- `whip NAME INDEX` - Prefer executable at INDEX (make it win)
- `whia NAME` - Show all matches with indices (shortcut for `whi -ia`)

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

## Options

### Flags

Short flags can be combined Unix-style (e.g., `-ai` = `-a -i`, `-ais` = `-a -i -s`).

- `-a, --all` - Show all PATH matches (default: only winner)
- `-f, --full` - Show full PATH directory listing
- `-i, --index` - Show PATH index next to each entry
- `-l, -L, --follow-symlinks` - Resolve and show canonical targets
- `-o, --one` - Only print the first match per name
- `-s, --stat` - Include inode/device/mtime/size metadata
- `-0, --print0` - NUL-separated output for use with xargs
- `-q, --quiet` - Suppress non-fatal stderr warnings
- `--silent` - Print nothing to stderr, use exit codes only
- `--show-nonexec` - Also list files that exist but aren't executable
- `-h, --help` - Print help information

### PATH Manipulation

- `--move <FROM> <TO>` - Move PATH entry from index FROM to index TO
- `--swap <IDX1> <IDX2>` - Swap PATH entries at indices IDX1 and IDX2
- `--prefer <NAME> <INDEX>` - Make executable NAME at INDEX win

### Options

- `--path <PATH>` - Override environment PATH string
- `--color <WHEN>` - Colorize output: auto, never, always [default: auto]

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