# whicha

List **all** PATH hits for a command with their PATH indices.

`whicha` is a tiny CLI tool that scans your PATH and shows you every executable match for one or more names. It can show PATH indices, follow symlinks, display file metadata, and list the complete PATH.

## Features

- **See all matches**: Unlike `which`, shows every executable in PATH, not just the first
- **Winner indication**: Clearly marks which executable would actually run with color
- **PATH indices** (`-i`): Shows the PATH index for each match
- **Full PATH listing** (`-f`): Displays complete PATH with indices
- **Follow symlinks** (`-l`): Resolves and shows canonical targets
- **File metadata** (`-s`): Shows inode, device, size, and modification time
- **Combinable flags**: Unix-style flag combining (e.g., `-isl`, `-of`)
- **Pipe-friendly**: Quiet by default, all output to stdout
- **Unix-focused**: Clean implementation for Unix-like systems
- **Zero dependencies**: Pure Rust standard library only

## Installation

```bash
cargo install whicha
```

Or build from source:

```bash
git clone https://github.com/alexykn/whicha
cd whicha
cargo build --release
```

## Usage

Basic usage - show all matches:

```bash
$ whicha python
/usr/local/bin/python
/usr/bin/python
```

With PATH indices:

```bash
$ whicha -i python
[1] /usr/local/bin/python
[23] /usr/bin/python
```

Show full PATH listing (flags can be combined):

```bash
$ whicha python -if

[1] /usr/local/bin/python
[23] /usr/bin/python

[1] /opt/homebrew/bin
[2] /opt/homebrew/sbin
...
[23] /usr/bin
...
[72] /Applications/Ghostty.app/Contents/MacOS
```

Multiple names:

```bash
$ whicha python node cargo
/usr/local/bin/python
/usr/bin/python
/usr/local/bin/node
/opt/homebrew/bin/cargo
/usr/local/bin/cargo
```

Show only the winner with `--one`:

```bash
$ whicha --one python
/usr/local/bin/python
```

Read names from stdin:

```bash
$ echo -e "python\\nnode\\ncargo" | whicha
```

Follow symlinks (shows original → canonical):

```bash
$ whicha -L cargo
/opt/homebrew/bin/cargo → /opt/homebrew/Cellar/rustup/1.28.2/bin/rustup-init
/Users/user/.cargo/bin/cargo → /Users/user/.cargo/bin/rustup
/Users/user/.rustup/toolchains/stable-aarch64-apple-darwin/bin/cargo
```

Override PATH:

```bash
$ whicha --path="/usr/local/bin:/usr/bin" python
```

## Options

### Flags

Short flags can be combined Unix-style (e.g., `-if` = `-i -f`, `-isl` = `-i -s -l`).

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
$ whicha python python3 python3.11
```

Check which node would run with indices:

```bash
$ whicha -i node
```

Use with xargs to check executables:

```bash
$ whicha -0 --one python node cargo | xargs -0 -n1 file
```

Find all versions with metadata:

```bash
$ whicha -is gcc
```

## Comparison with `which`

| Feature | `which` | `whicha` |
|---------|---------|----------|
| Show first match | ✓ | ✓ |
| Show all matches | Some versions with `-a` | ✓ Always |
| Show PATH indices | ✗ | ✓ With `-i` |
| Full PATH listing | ✗ | ✓ With `-f` |
| Follow symlinks | Some versions | ✓ With `-l/-L` |
| File metadata | ✗ | ✓ With `-s` |
| Combinable flags | ✗ | ✓ |
| Multiple names | ✓ | ✓ |
| Stdin input | ✗ | ✓ |
| Pipe-friendly | Varies | ✓ |

## Why?

I wanted to know:
- What other versions of a command exist on my PATH?
- Which PATH directory does each one come from?
- What's my complete PATH?

`which -a` shows all matches on some systems, but doesn't show PATH indices or provide the full PATH context. `whicha` gives you the complete picture with simple, combinable flags.

## License

MIT

## Author

Alexander Knott <alexander.knott@posteo.de>