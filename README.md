# whi

`whi` is a smarter `which` that also lets you rearrange your current shell’s `PATH` safely. Install the shell integration once, run the high level commands (`prefer`, `move`, `switch`, `clean`, `delete`, `diff`, `save`), and grab the helper shortcuts if you like terse aliases.

> **Heads up**: mutation commands only run after the integration exports `WHI_SHELL_INITIALIZED=1`. If you see the integration warning, run the snippet for your shell and add it to your config for persistence.

## Install & Integrate

```bash
cargo install whi
```

Pick the snippet for your shell and paste it into a terminal. Add the same line to your shell config so it runs on startup.

```bash
# Bash
eval "$(whi init bash)"      # add to ~/.bashrc (or the file you source)

# Zsh
eval "$(whi init zsh)"       # add to ~/.zshrc

# Fish
whi init fish | source        # add to ~/.config/fish/config.fish
```

The snippet defines helper functions and exports `WHI_SHELL_INITIALIZED=1` so the `whi` binary knows it is safe to mutate the live `PATH`.

## Core Commands

All of these operate on the current shell session. Each command prints the updated `PATH`; the integration captures the string and updates `PATH` for you.

```bash
# Which like usage
whi cargo
whi -n cargo (no index, same output which gives)
whi -a cargo (shorthand whia)
whi -an cargo

# Show full path (line seperated)
whi -f
whi -fn

# Prefer a specific executable or add a path
# Makes the minimal required change to path / puts it at
# the lowest spot in path that achieves it being the winner
whi prefer cargo 5
whi prefer cargo ~/.cargo/bin
whi prefer cargo github release
whi prefer ~/.local/bin (except this, no executable provided -> acts like fish_add_paht)


# Move and swap entries by index (1-based)
whi move 12 1
whi switch 4 9

# Clean duplicates
whi clean

# Delete entries
whi delete 3 9 (specific, multiple)
whi delete ~/.local/bin (specific, single)
whi delete build temp (fuzzy all paths that match)

# Inspect and persist PATH state
whi diff              # current shell
whi diff zsh          # another shell
whi diff full         # shorthand for --full
whi save              # current shell
whi save fish         # explicit
whi save all          # every shell
```

### Querying executables

```bash
whi node                 # show the first match (with PATH index)
whi --all node           # list every match

# This is cool ... list all matches, newline, list full path
# with all path entries containing the binary highlighted
whi --full cargo
```

Useful flags:
- `-a/--all`
- `-f/--full`
- `-l/--follow-symlinks`
- `-s/--stat`
- `-0/--print0`
- `-q/--quiet`
- `--silent`
- `--color <auto|never|always>`
- `--path <PATH>`

`whi --help` shows the verbs (`diff`, `save`, `prefer`, `move`, `switch`, `clean`, `delete`). The integration intercepts those public names and rewrites them to the hidden `__…` subcommands that actually mutate the environment.

## Helper Shortcuts

These are defined by the integration and map directly to the core commands:

```bash
whip cargo github release    # -> whi prefer ...
whim 12 1                    # -> whi move ...
whis 4 9                     # -> whi switch ...
whic                         # -> whi clean
whid 3 9                     # -> whi delete ...
whia python                  # -> whi --all python
```

Use whichever spelling you prefer—both routes converge in Rust.

## Persistence & Logs

- Saved PATH files live in `~/.whi/` (`saved_path_bash`, `saved_path_zsh`, `saved_path_fish`). `whi save all` refreshes all three. Each save creates a `*.bak` backup before overwriting.
- Session logs live in `${XDG_RUNTIME_DIR:-$TMPDIR}/whi-<uid>/session_<ppid>.log`. They track explicit moves/deletions so `whi diff` can distinguish explicit vs. implicit changes. When more than 30 logs accumulate, the oldest ones are pruned automatically.

## Notes

- Mutating commands exit with an instructional message if the integration is missing. Copy the snippet shown (or the table above), run it, and add it to your shell config for future sessions.
- If you want to script against `whi` directly, capture stdout and export the string yourself. The integration just automates that for interactive use.

## License

MIT
