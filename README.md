# whi

`whi` is a smarter `which` that also lets you rearrange your current shell's `PATH` safely. Install the shell integration once, run the high level commands (`prefer`, `add`, `move`, `switch`, `clean`, `delete`, `undo`, `redo`, `reset`, `diff`, `apply`, `save`, `load`, `list`, `rmp`, `file`, `source`, `exit`, `var`, `shorthands`), and grab the helper shortcuts if you like terse aliases.

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
- Defines helper functions (`whip`, `whiad`, `whia`, `whim`, `whis`, `whiu`, `whir`, `whil`, `whiv`, `whish`, etc.)
- Exports `WHI_SHELL_INITIALIZED=1` so `whi` knows it's safe to mutate PATH
- Captures the final PATH as your session baseline for undo/diff tracking

## Core Commands

All of these operate on the current shell session. Each command prints the updated `PATH`; the integration captures the string and updates `PATH` for you.

### Querying executables

```bash
whi cargo                    # show the first match (with PATH index)
whi -n cargo                 # no index, same output which gives
whi -a cargo                 # show all matches
# Shorthand: whia
whi -an cargo                # all matches, no index

# Fuzzy search fallback (if exact match fails)
whi carg                     # finds "cargo" via fuzzy matching

# Show full path (line separated)
whi -f                       # list all PATH entries
whi -fn                      # list all PATH entries, no index

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

### PATH manipulation

```bash
# Add paths to PATH (prepends by default)
whi add ~/.local/bin                 # add single path
whi add ~/bin ~/.cargo/bin           # add multiple paths
# Shorthand: whiad

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
```

### History & state management

```bash
# Undo/redo/reset PATH changes
whi undo                     # undo last operation
whi undo 3                   # undo last 3 operations
whi redo                     # redo next operation
whi redo 2                   # redo next 2 operations
whi reset                    # reset to initial session state

# Inspect PATH changes
whi diff                     # show changes since session start
whi diff full                # show all entries (including unchanged)
```

### Persistence & profiles

```bash
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

### Environment variables

```bash
# Query environment variables
whi var PATH                 # show PATH variable (exact match, case-insensitive)
whi var cargo                # fuzzy search for variables matching 'cargo'
whi var -f                   # list all environment variables (sorted)
# Shorthand: whiv

# Show all available shortcuts
whi shorthands               # display table of all whi* shorthands
# Shorthand: whish
```

### Virtual environments (venv)

`whi` can create project-specific PATH environments similar to Python virtualenvs or direnv, but for PATH management. This is perfect for projects that need specific tool versions or custom PATH configurations.

```bash
# Create whifile from current PATH (like requirements.txt for PATH)
whi file                     # create whifile in current directory
whi file -f                  # force overwrite existing whifile

# Activate venv (read whifile and switch PATH)
whi source                   # activate venv from ./whifile
# Shell shows: [dirname] user@host:~/project $

# Exit venv (restore previous PATH)
whi exit                     # deactivate and restore PATH
```

**How it works:**
- `whi file` snapshots your current PATH into a `whifile` in the current directory
- `whi source` reads `whifile` and replaces your PATH (saves old PATH for restore)
- `whi exit` restores your previous PATH
- Your shell prompt shows `[venv-name]` when active (like Python venvs)
- All PATH operations (`prefer`, `move`, `delete`, etc.) work normally inside venvs
- Venv state is session-specific (different terminals = different venv states)

**Use cases:**
- Lock tool versions per project (e.g., specific Node, Python, Ruby versions)
- Isolate project-specific binaries from global PATH
- Test PATH configurations before applying globally
- Share reproducible development environments via version control

**Auto-activation:**
You can enable auto-activation in `~/.whi/config.toml`:
```toml
[venv]
auto_activate_file = true   # automatically source whifile when entering directories
auto_deactivate_file = true # automatically run whi exit (and extra exit hooks) when leaving
```

When enabled, the shell integration will automatically activate venvs when you `cd` into directories containing `whifile` and automatically run `whi exit` (including `$source … <exit_command>` hooks and `$pyenv` deactivation) when you leave.

> **Known Issue:** Auto-activation currently does not work in Zsh. Bash and Fish work correctly. For Zsh users, please manually run `whi source` when entering directories with a `whifile`. This will be fixed in a future release.

**whifile format (v2 / 0.6.0+):**
The file uses a directive-based format with multiple PATH and ENV strategies:
```
# Replace session PATH entirely
!path.replace
/usr/local/bin
/usr/bin
/bin
~/custom/bin
/Users/$USER/.local/bin

# Or prepend to session PATH
# !path.prepend
# ~/my-tools/bin

# Or append to session PATH
# !path.append
# ~/extra/bin

# Set environment variables
!env.set
# Comments are supported
RUST_LOG debug
PROJECT_ROOT $(pwd)
CONFIG_DIR $HOME/.config/myapp
MY_VAR hello world

# Or replace all env vars (whitelist mode)
# !env.replace
# KEEP_THIS value
# AND_THIS value

# Or unset specific vars
# !env.unset
# REMOVE_THIS
# AND_THIS
```

**PATH directives (mutually exclusive):**
- `!path.replace` - Replace session PATH entirely with listed paths
- `!path.prepend` - Prepend paths to session PATH
- `!path.append` - Append paths to session PATH
- Each path is on its own line
- Supports shell variable expansion: `$VAR`, `${VAR}`, `~`, `$(command)`
- Variables are expanded when sourcing the venv

**ENV directives:**
- `!env.set` - Set specific environment variables (default)
- `!env.replace` - Replace all env vars (whitelist mode, auto-unsets others)
- `!env.unset` - Unset specific variables
- Fish-style syntax: `KEY value` (space-separated, no `=` or quotes needed)
- Supports shell variable expansion: `$VAR`, `${VAR}`, `~`, `$(command)`, `` `command` ``
- Values can contain spaces, special characters (`:`, `=`, `/`, etc.)
- Comments start with `#`
- Variables are set when entering venv, unset when exiting

**Extra directives (optional):**
- `!whi.extra` - Source scripts or activate Python virtual environments
- Executed AFTER `!path` and `!env` sections to prevent interference
- Two directive types:
  - `$source /path/to/script` – Source any shell script (user's responsibility for shell compatibility). Append an optional exit command to run during `whi exit`: `$source /path/to/script cleanup_command --flag`. The exit command runs before Whi unsets venv variables so you can undo whatever the script configured.
  - `$pyenv /path/to/venv` – Activate Python venv (auto-detects shell and sources activate/activate.fish). Whi keeps a guard around the virtualenv so calling `deactivate` directly prints `environment managed by whi...`; use `whi exit` (or auto-deactivate) so Whi can restore history and PATH correctly. Regular `$source` directives do **not** install this guard.
- Supports shell variable expansion: `$VAR`, `${VAR}`, `~`, `$(command)`
- Python venv example: `$pyenv $(pwd)/.venv` or `$pyenv .venv` (both work)
- On `whi exit`, runs `deactivate` for Python venvs automatically and executes any `$source … <exit_command>` hooks you defined
- With `[venv] auto_activate_file`/`auto_deactivate_file` enabled in `config.toml`, the shell integration automatically applies `$pyenv`/`$source` directives (and their exit commands) when you `cd` into or out of a whifile directory.

Example:
```
!whi.extra
$pyenv $(pwd)/.venv
$source ~/.config/project-setup.sh
```

**Legacy format support:**
Files with `PATH!` and `ENV!` sections (pre-0.6.0) are automatically converted to `!path.replace` and `!env.set` for backward compatibility.

### Shell prompt behaviour

Whi uses virtualenv's standard environment variables (`VIRTUAL_ENV` and `VIRTUAL_ENV_PROMPT`) for venv indication. Modern prompt frameworks (Starship, oh-my-posh, Tide, powerlevel10k, etc.) automatically detect these variables and display the venv indicator in their own style - no manual configuration needed!

`whi --help` shows the verbs (`prefer`, `add`, `move`, `switch`, `clean`, `delete`, `undo`, `redo`, `reset`, `diff`, `apply`, `save`, `load`, `list`, `rmp`, `file`, `source`, `exit`, `var`, `shorthands`). The integration intercepts those public names and rewrites them to the hidden `__…` subcommands that actually mutate the environment.

## Helper Shortcuts

These are defined by the integration and map directly to the core commands:

```bash
# whip: Swiss Army knife for PATH management
whip cargo 5                 # prefer by index
whip cargo ~/.cargo/bin      # prefer by path (adds if needed!)
whip cargo toolchain stable  # prefer by fuzzy pattern
whip ~/.local/bin            # add path to PATH

# Other shortcuts
whia cargo                   # -> whi --all cargo
whiad ~/.local/bin ~/bin     # -> whi add ...
whim 12 1                    # -> whi move ...
whis 4 9                     # -> whi switch ...
whic                         # -> whi clean
whid 3 9                     # -> whi delete ...
whiu                         # -> whi undo
whiu 3                       # -> whi undo 3
whir                         # -> whi redo
whir 2                       # -> whi redo 2
whil work                    # -> whi load work
whiv PATH                    # -> whi var PATH
whiv -f                      # -> whi var -f (list all env vars)
whish                        # -> whi shorthands (show all shortcuts)
```

Use whichever spelling you prefer—both routes converge in Rust.

## Persistence & State

- **Saved PATH files** live in `~/.whi/` (`saved_path_bash`, `saved_path_zsh`, `saved_path_fish`). `whi apply all` writes to all three. Each save creates a `*.bak` backup before overwriting. Files use a human-friendly directive format with `!path.replace` and `!env.set` sections (v0.6.0+). Legacy `PATH!`/`ENV!` format (pre-0.6.0) and colon-separated files (pre-0.6.0) are automatically detected and supported for backward compatibility.

- **Profile storage** lives in `~/.whi/profiles/`. Each profile is a file in the same human-friendly format as saved PATH files. Use `whi save <name>` to save current PATH as a profile, `whi load <name>` to restore it, and `whi list` to see all profiles. You can manually edit these files - use `!path.replace`/`!path.prepend`/`!path.append` for PATH directives and `!env.set`/`!env.replace`/`!env.unset` for environment variables (all support shell variable expansion).

- **Configuration** lives in `~/.whi/config.toml`. Auto-created on first run with defaults. Controls venv auto-activation and protected paths (preserved during `whi apply` to prevent breaking your shell).

- **Session snapshots** live in `${XDG_RUNTIME_DIR:-/tmp}/whi-<uid>/session_<ppid>.log` (or `session_<ppid>/<venv-dir-hash>.log` when in a venv). Each PATH modification writes a snapshot (timestamp + full PATH string). The undo/redo system navigates through these snapshots with a cursor. Sessions keep up to 500 snapshots (initial + last 499) and auto-cleanup old sessions after 24 hours.

- **Undo cursor** lives in `${XDG_RUNTIME_DIR:-/tmp}/whi-<uid>/session_<ppid>.cursor` (or `session_<ppid>/<venv-dir-hash>.cursor` when in a venv). Tracks your position in the snapshot history. No cursor file means you're at the latest state.

- **Venv state** (when active) is stored in `${XDG_RUNTIME_DIR:-/tmp}/whi-<uid>/session_<ppid>/venv_restore` (PATH to restore on exit), `venv_dir` (venv directory path), and `venv_exit_commands` (any `$source … <exit_command>` hooks). This is session-specific - each terminal has independent venv state.

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

## Acknowledgments

- Prompt integration approach inspired by [virtualenv](https://github.com/pypa/virtualenv). Thanks to the virtualenv team for their battle-tested solution using `VIRTUAL_ENV` and `VIRTUAL_ENV_PROMPT` environment variables that works seamlessly with modern prompt frameworks (Starship, oh-my-posh, Tide, etc.).
- The "whifile" name was inspired by [just](https://github.com/casey/just)'s "justfile" naming convention.
