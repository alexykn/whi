# Architecture Notes

## Current scope

Whi is a PATH management tool.

- It queries executables on PATH.
- It mutates PATH within the current shell via shell integration.
- It persists PATH snapshots and named PATH profiles.
- It no longer manages environment variables, `whifile` activation, or virtual environments.

## Key modules

- `src/bin/whi.rs`: CLI entrypoint and hidden subcommands used by shell integration
- `src/app.rs`: core PATH operations and history handling
- `src/path_file.rs`: path-only file parser/formatter with read-only compatibility for deprecated env directives
- `src/config_manager.rs`: saved PATH and profile persistence
- `src/protected_config.rs`: protected path loading and migration from old config
- `src/shell_integration.rs`: shell init script generation
- `src/history.rs` and `src/session_tracker.rs`: session-scoped PATH history for undo/redo/diff/reset

## Shell integration model

- Public mutating commands still require shell integration.
- The shell function intercepts PATH-mutating commands and calls the matching hidden `__...` subcommand.
- Hidden subcommands print the new PATH, and the shell function exports it into the current shell session.

## Compatibility policy

- Old saved files and profiles containing `!env.*`, `!whi.extra`, or legacy `ENV!` sections are read-only compatible.
- Deprecated directives are ignored with a warning.
- Old files are never rewritten automatically.
