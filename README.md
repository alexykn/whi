# whicha

List **all** PATH hits for a command and explain why one wins.

`whicha` is a tiny CLI tool that scans your PATH and shows you every executable match for one or more names. It clearly indicates which candidate would be chosen first (the winner) and can provide detailed reasoning with the `-e/--explain` flag.

## Features

- **See all matches**: Unlike `which`, shows every executable in PATH, not just the first
- **Winner indication**: Clearly marks which executable would actually run with color
- **Explain mode** (`-e`): Shows hits with PATH indices in a compact format
- **Full mode** (`-e -f`): Complete PATH listing with detailed reasoning
- **Pipe-friendly**: Quiet by default, results to stdout, explanations to stderr
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

With explanation (hits-only mode):

```bash
$ whicha -e python
/usr/local/bin/python
/usr/bin/python

[whicha] python
  [1] /usr/local/bin/python (winner)
  [23] /usr/bin/python (shadowed)

Winner: earliest PATH hit is [#1].

```

Full explanation with complete PATH listing (flags can be combined):

```bash
$ whicha -ef gcc    # same as -e -f

[whicha] gcc

PATH (72 entries)
  [1] /opt/homebrew/bin
  [2] /opt/homebrew/sbin
  ...
  [23] /opt/pm/live/bin
  ...
  [61] /usr/bin
  ...
  [72] /Applications/Ghostty.app/Contents/MacOS

Found:
  [23] /opt/pm/live/bin/gcc (winner)
  [61] /usr/bin/gcc (shadowed)

Winner: earliest PATH hit is [#23].

Note: shell aliases/functions/builtins aren't visible to whicha.
      Use 'type -a gcc' in your shell to see all matches.

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

Show only the winner with `--first`:

```bash
$ whicha --first python
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

Short flags can be combined Unix-style (e.g., `-ef` = `-e -f`, `-eL` = `-e -L`).

- `-e, --explain` - Show hits with PATH indices to stderr
- `-f, --full` - With `-e`: include full PATH directory listing
- `-L, --follow-symlinks` - Resolve and show canonical targets
- `-0, --print0` - NUL-separated output for use with xargs
- `-q, --quiet` - Suppress non-fatal stderr warnings
- `--silent` - Print nothing to stderr, use exit codes only
- `--first` - Only print the first match per name
- `--show-nonexec` - Also list files that exist but aren't executable
- `--stat` - Include inode/device/mtime/size in --explain
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

Check which node would run and why:

```bash
$ whicha -e node
```

Use with xargs to check executables:

```bash
$ whicha -0 --first python node cargo | xargs -0 -n1 file
```

Find shadowed commands:

```bash
$ whicha -e gcc | grep shadowed
```

## Comparison with `which`

| Feature | `which` | `whicha` |
|---------|---------|----------|
| Show first match | ✓ | ✓ |
| Show all matches | Some versions with `-a` | ✓ Always |
| Explain why | ✗ | ✓ With `-e` |
| Follow symlinks | Some versions | ✓ With `-L` |
| Multiple names | ✓ | ✓ |
| Stdin input | ✗ | ✓ |
| Pipe-friendly | Varies | ✓ |

## Why?

I wanted to know:
- What other versions of a command exist on my PATH?
- Which one would actually run?
- Why is *that* one chosen over the others?

`which -a` shows all matches on some systems, but doesn't explain the selection logic or provide structured output for scripting. `whicha` does both.

## License

MIT

## Author

Alexander Knott <alexander.knott@posteo.de>