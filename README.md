# whi

`whi` is a smarter `which` that also lets you rearrange your current shell's `PATH` safely.

Whi is path-only again.

- Managed features: `prefer`, `add`, `move`, `switch`, `clean`, `delete`, `undo`, `redo`, `reset`, `diff`, `apply`, `save`, `load`, `list`, `rmp`, `shorthands`
- Removed features: environment-variable management, `whifile` activation, and virtual environment management
- Compatibility: old profile and saved PATH files that still contain `!env.*`, `!whi.extra`, or legacy `ENV!` sections are still readable, but those directives are ignored with a warning and are never rewritten automatically

## Install shell integration

```sh
eval "$(whi init bash)"      # add to the end of ~/.bashrc
eval "$(whi init zsh)"       # add to the end of ~/.zshrc
whi init fish | source        # add to the end of ~/.config/fish/config.fish
```

## Common commands

```sh
whi cargo                     # find cargo on PATH
whi --all cargo               # show all matches
whi prefer cargo 2            # make PATH entry 2 win for cargo
whi prefer ~/.cargo/bin       # prepend a path to PATH if needed
whi add ~/.local/bin          # add one or more paths
whi move 5 2                  # move PATH entry 5 to 2
whi switch 2 3                # swap PATH entries
whi clean                     # remove duplicate PATH entries
whi delete 7                  # delete PATH entry 7
whi delete cargo              # delete PATH entries matching pattern/path
whi undo                      # undo last PATH change
whi redo                      # redo last PATH change
whi reset                     # reset to initial PATH for this shell session
whi diff                      # show PATH changes since session start
```

## Persist PATH

```sh
whi apply                     # save current PATH for this shell
whi apply fish                # save for a specific shell
whi apply all                 # save for bash, zsh, and fish

whi save work                 # save current PATH as profile "work"
whi load work                 # load profile "work"
whi list                      # list saved profiles
whi rmp work                  # remove profile "work"
```

## File format

Saved PATH files and profiles use a path-only directive format:

```text
!path.replace
/usr/local/bin
/usr/bin
/bin
```

Supported directives:

- `!path.replace`
- `!path.prepend`
- `!path.append`
- legacy `PATH!`
- legacy colon-separated PATH strings

Deprecated directives are still parsed for compatibility and ignored:

- `!env.set`
- `!env.replace`
- `!env.unset`
- `!whi.extra`
- legacy `ENV!`

## Shorthands

```text
whip   -> whi prefer
whim   -> whi move
whis   -> whi switch
whic   -> whi clean
whid   -> whi delete
whia   -> whi --all
whiad  -> whi add
whiu   -> whi undo
whir   -> whi redo
whil   -> whi load
whish  -> whi shorthands
```

## Storage

- Saved PATH files: `~/.whi/saved_path_bash`, `~/.whi/saved_path_zsh`, `~/.whi/saved_path_fish`
- Profiles: `~/.whi/profiles/`
- Config: `~/.whi/config.toml`
- Protected paths: `~/.whi/protected_paths`
- Session history: `${XDG_RUNTIME_DIR:-/tmp}/whi-<uid>/session_<pid>.*`

## Notes

- Mutating commands require shell integration because they must update the current shell's `PATH`.
- `whi apply` preserves protected paths by default. Use `--no-protect` to skip that safety behavior.
