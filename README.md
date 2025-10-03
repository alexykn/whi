# whi

`whi` is a smarter `which` that also lets you rearrange your current shell's `PATH` safely. Install the shell integration once, run the high level commands (`prefer`, `move`, `switch`, `clean`, `delete`, `undo`, `redo`, `reset`, `diff`, `apply`, `save`, `load`), and grab the helper shortcuts if you like terse aliases.

> **Heads up**: mutation commands only run after the integration exports `WHI_SHELL_INITIALIZED=1`. If you see the integration warning, run the snippet for your shell and add it to your config for persistence.

## Install & Integrate

```bash
cargo install whi
```

Pick the snippet for your shell and paste it into a terminal. **Add the same line to the END of your shell config** so it runs on startup after all PATH modifications.

```bash
# Bash
eval "$(whi init bash)"      # add to the END of ~/.bashrc (or the file you source)

# Zsh
eval "$(whi init zsh)"       # add to the END of ~/.zshrc

# Fish
whi init fish | source        # add to the END of ~/.config/fish/config.fish
```

**Important:** The integration must be at the END of your config so `whi` captures your final PATH after all modifications (homebrew, cargo, etc.).

The snippet:
- Loads any previously saved PATH (from `~/.whi/saved_path_*`)
- Defines helper functions (`whip`, `whim`, `whis`, `whiu`, `whir`, `whil`, etc.)
- Exports `WHI_SHELL_INITIALIZED=1` so `whi` knows it's safe to mutate PATH
- Captures the final PATH as your session baseline for undo/diff tracking

## Core Commands

All of these operate on the current shell session. Each command prints the updated `PATH`; the integration captures the string and updates `PATH` for you.

```bash
# Which like usage
whi cargo
whi -n cargo                 # no index, same output which gives
whi -a cargo                 # shorthand: whia
whi -an cargo

# Show full path (line separated)
whi -f
whi -fn

# Prefer: make an executable win (or add a path)
# The Swiss Army knife - works with index, path, or fuzzy pattern
# Makes minimal changes to achieve the goal

whi prefer cargo 5                   # prefer by index
whi prefer cargo ~/.cargo/bin        # prefer by exact path (moves if present)
whi prefer cargo ~/new/rust/bin      # adds path if not in PATH (validates cargo exists!)
whi prefer cargo toolchain stable    # prefer by fuzzy pattern
whi prefer bat github release        # another fuzzy example
whi prefer ~/.local/bin              # no executable -> add path (like fish_add_path)

# Move and swap entries by index (1-based)
whi move 12 1
whi switch 4 9

# Clean duplicates
whi clean

# Delete entries
whi delete 3 9               # specific indices
whi delete ~/.local/bin      # exact path
whi delete build temp        # fuzzy - all paths matching pattern

# Undo/redo/reset PATH changes
whi undo                     # undo last operation
whi undo 3                   # undo last 3 operations
whi redo                     # redo next operation
whi redo 2                   # redo next 2 operations
whi reset                    # reset to initial session state

# Inspect PATH changes
whi diff                     # show changes since session start
whi diff full                # show all entries (including unchanged)

# Persist PATH to shell config files
whi apply                    # save to current shell's config
whi apply fish               # save to specific shell
whi apply all                # save to all shells (bash/zsh/fish)
whi apply --force [--no-protect]   # required when running inside an active whi venv

# Profile management
whi save work                # save current PATH as profile "work"
whi load work                # load profile "work"
whi list                     # list all saved profiles
whi rmp work                 # remove profile "work"
```

### Querying executables

```bash
whi node                     # show the first match (with PATH index)
whi --all node               # list every match

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

`whi --help` shows the verbs (`prefer`, `move`, `switch`, `clean`, `delete`, `undo`, `redo`, `reset`, `diff`, `apply`, `save`, `load`, `list`, `rmp`). The integration intercepts those public names and rewrites them to the hidden `__…` subcommands that actually mutate the environment.

## Helper Shortcuts

These are defined by the integration and map directly to the core commands:

```bash
# whip: Swiss Army knife for PATH management
whip cargo 5                 # prefer by index
whip cargo ~/.cargo/bin      # prefer by path (adds if needed!)
whip cargo toolchain stable  # prefer by fuzzy pattern
whip ~/.local/bin            # add path to PATH

# Other shortcuts
whim 12 1                    # -> whi move ...
whis 4 9                     # -> whi switch ...
whic                         # -> whi clean
whid 3 9                     # -> whi delete ...
whiu                         # -> whi undo
whiu 3                       # -> whi undo 3
whir                         # -> whi redo
whir 2                       # -> whi redo 2
whil work                    # -> whi load work
whia python                  # -> whi --all python
```

Use whichever spelling you prefer—both routes converge in Rust.

## Persistence & State

- **Saved PATH files** live in `~/.whi/` (`saved_path_bash`, `saved_path_zsh`, `saved_path_fish`). `whi apply all` writes to all three. Each save creates a `*.bak` backup before overwriting. Files use a human-friendly format with `PATH!` and `ENV!` sections (one path per line). Legacy colon-separated files from pre-0.5.0 are automatically detected and supported for backward compatibility.

- **Profile storage** lives in `~/.whi/profiles/`. Each profile is a file in the same human-friendly format as saved PATH files. Use `whi save <name>` to save current PATH as a profile, `whi load <name>` to restore it, and `whi list` to see all profiles. You can manually edit these files - just list one path per line under the `PATH!` section.

- **Session snapshots** live in `${XDG_RUNTIME_DIR:-/tmp}/whi-<uid>/session_<ppid>.log`. Each PATH modification writes a snapshot (timestamp + full PATH string). The undo/redo system navigates through these snapshots with a cursor. Sessions keep up to 500 snapshots (initial + last 499) and auto-cleanup old sessions after 24 hours.

- **Undo cursor** lives in `${XDG_RUNTIME_DIR:-/tmp}/whi-<uid>/session_<ppid>.cursor`. Tracks your position in the snapshot history. No cursor file means you're at the latest state.

## How undo/redo works

Every PATH mutation writes a snapshot. The undo system navigates backwards through snapshots, and redo moves forward. If you undo then make a new change, the "future" timeline is discarded (standard undo/redo behavior).

```bash
# Example session:
$ whi move 5 1              # snapshot 1 (initial is snapshot 0)
$ whi delete 3              # snapshot 2
$ whi undo                  # cursor moves to snapshot 1
$ whi undo                  # cursor moves to snapshot 0 (initial)
$ whi redo                  # cursor moves to snapshot 1
$ whi prefer cargo 2        # snapshot 3 (snapshot 2 discarded, new timeline)
$ whi reset                 # back to snapshot 0, cursor cleared
```

`whi diff` compares your current PATH to the initial session snapshot, so it shows **all** changes including manual `export PATH=...` modifications (not just whi operations).

## Notes

- Mutating commands exit with an instructional message if the integration is missing. Copy the snippet shown, run it, and add it to your shell config for future sessions.

- The shell integration uses absolute paths to `whi`, so it continues to work even if PATH is modified or replaced during initialization.

- If you want to script against `whi` directly, capture stdout and export the string yourself. The integration just automates that for interactive use.

## License

MIT License. See [LICENSE](LICENSE) for details.
