# whi shell integration for bash/zsh (v0.6.6)

# Absolute path to the whi binary is injected by `whi init`
__WHI_BIN="__WHI_BIN__"

__WHI_CONFIG_PATH="${HOME}/.whi/config.toml"
__WHI_AUTO_FILE=0
__WHI_AUTO_FILE_MTIME=""
__WHI_LAST_PWD=""
__WHI_LAST_HAS_FILE=0
__WHI_STAT_STYLE=""

__whi_exec() {
    "$__WHI_BIN" "$@"
}

__whi_detect_stat_style() {
    [ -n "$__WHI_STAT_STYLE" ] && return 0

    local probe
    probe="${HOME:-.}"

    if stat -c %Y "$probe" >/dev/null 2>&1; then
        __WHI_STAT_STYLE="gnu"
    elif stat -f %m "$probe" >/dev/null 2>&1; then
        __WHI_STAT_STYLE="bsd"
    else
        __WHI_STAT_STYLE="none"
    fi
}

__whi_stat_mtime() {
    local path="$1"

    [ -n "$path" ] && [ -e "$path" ] || return 0

    __whi_detect_stat_style

    local result=""
    case "$__WHI_STAT_STYLE" in
        gnu)
            result=$(stat -c %Y "$path" 2>/dev/null) || result=""
            ;;
        bsd)
            result=$(stat -f %m "$path" 2>/dev/null) || result=""
            ;;
        *)
            :
            ;;
    esac

    [ -n "$result" ] && printf '%s\n' "$result"
    return 0
}

__whi_refresh_auto_config() {
    local current_mtime=""

    if [ -n "$__WHI_CONFIG_PATH" ]; then
        current_mtime=$(__whi_stat_mtime "$__WHI_CONFIG_PATH")
    fi

    if [ "${__WHI_AUTO_CONFIG_LOADED:-0}" = "1" ] && [ "$current_mtime" = "${__WHI_AUTO_FILE_MTIME-}" ]; then
        return 0
    fi

    local output
    output=$(__whi_exec __should_auto_activate 2>/dev/null || printf 'file=0\ndeactivate=0')

    __WHI_AUTO_FILE=0
    __WHI_AUTO_DEACTIVATE=0

    while IFS= read -r line; do
        case "$line" in
            file=1) __WHI_AUTO_FILE=1 ;;
            file=0) __WHI_AUTO_FILE=0 ;;
            deactivate=1) __WHI_AUTO_DEACTIVATE=1 ;;
            deactivate=0) __WHI_AUTO_DEACTIVATE=0 ;;
        esac
    done <<< "$output"

    if [ -z "$current_mtime" ]; then
        __WHI_AUTO_FILE=0
    fi

    __WHI_AUTO_FILE_MTIME="$current_mtime"
    __WHI_AUTO_CONFIG_LOADED=1
}

__whi_install_deactivate_guard() {
    if [ "${_WHI_PYENV_GUARD_INSTALLED:-}" = "1" ]; then
        return
    fi

    if ! command -v deactivate >/dev/null 2>&1; then
        return
    fi

    # Save original deactivate function
    if [ -n "$BASH_VERSION" ]; then
        eval "$(declare -f deactivate | sed '1s/deactivate/__whi_original_deactivate/')"
    elif [ -n "$ZSH_VERSION" ]; then
        functions[__whi_original_deactivate]="${functions[deactivate]}"
    fi

    # Replace with guarded version
    deactivate() {
        if [ -n "${WHI_ALLOW_DEACTIVATE:-}" ]; then
            __whi_original_deactivate "$@"
        else
            echo "environment managed by whi, please use 'whi exit' to leave" >&2
            return 1
        fi
    }

    _WHI_PYENV_GUARD_INSTALLED=1
}

__whi_remove_deactivate_guard() {
    if [ "${_WHI_PYENV_GUARD_INSTALLED:-}" != "1" ]; then
        return
    fi

    if command -v __whi_original_deactivate >/dev/null 2>&1; then
        # Restore original deactivate
        if [ -n "$BASH_VERSION" ]; then
            eval "$(declare -f __whi_original_deactivate | sed '1s/__whi_original_deactivate/deactivate/')"
        elif [ -n "$ZSH_VERSION" ]; then
            functions[deactivate]="${functions[__whi_original_deactivate]}"
        fi
        unset -f __whi_original_deactivate
    else
        # No original, remove deactivate entirely
        unset -f deactivate 2>/dev/null || true
    fi

    unset _WHI_PYENV_GUARD_INSTALLED
}

__whi_apply_transition() {
    local output
    if ! output="$(__whi_exec "$@")"; then
        return $?
    fi

    local tab=$'\t'
    local processed=0
    local install_guard=0
    local remove_guard=0
    local line rest var value

    while IFS= read -r line; do
        [ -z "$line" ] && continue
        case "$line" in
            PATH"$tab"*)
                export PATH="${line#PATH$tab}"
                processed=1
                ;;
            SET"$tab"*)
                rest=${line#SET$tab}
                var=${rest%%$tab*}
                value=${rest#*$tab}
                if [ "$rest" = "$value" ]; then
                    value=""
                fi
                if [ -n "$var" ]; then
                    export "$var=$value"
                    if [ "$var" = "WHI_PYENV_MANAGED" ]; then
                        install_guard=1
                    fi
                fi
                processed=1
                ;;
            UNSET"$tab"*)
                var=${line#UNSET$tab}
                if [ -n "$var" ]; then
                    unset "$var"
                    if [ "$var" = "WHI_PYENV_MANAGED" ]; then
                        remove_guard=1
                    fi
                fi
                processed=1
                ;;
            SOURCE"$tab"*)
                rest=${line#SOURCE$tab}
                if [ -n "$rest" ] && [ -f "$rest" ]; then
                    # Source the script
                    source "$rest"

                    # Note: VIRTUAL_ENV_PROMPT is already set by whi source, script might override it
                    # which is fine - but we don't need to do anything here

                    # Clean up if activate script created backup
                    unset _OLD_VIRTUAL_PS1 2>/dev/null || true
                fi
                processed=1
                ;;
            RUN"$tab"*)
                rest=${line#RUN$tab}
                if [ -n "$rest" ]; then
                    eval "$rest"
                fi
                processed=1
                ;;
            PYENV"$tab"*)
                rest=${line#PYENV$tab}
                if [ -n "$rest" ]; then
                    local venv_dir="$rest"

                    # Resolve to absolute path if needed
                    if [ "${venv_dir:0:1}" != "/" ]; then
                        venv_dir="$(pwd)/$venv_dir"
                    fi

                    # Normalize path (remove trailing slashes, handle .venv vs .venv/bin)
                    venv_dir="${venv_dir%/}"
                    if [ "${venv_dir##*/}" = "bin" ]; then
                        venv_dir="${venv_dir%/bin}"
                    fi

                    # Verify venv structure
                    if [ ! -d "$venv_dir" ]; then
                        echo "Error: Venv directory does not exist: $venv_dir" >&2
                    elif [ ! -d "$venv_dir/bin" ]; then
                        echo "Error: Not a valid Python venv (missing bin/): $venv_dir" >&2
                    elif [ ! -f "$venv_dir/bin/python" ] && [ ! -L "$venv_dir/bin/python" ]; then
                        echo "Error: Not a valid Python venv (missing bin/python): $venv_dir" >&2
                    else
                        # Store old environment for restoration
                        export _WHI_OLD_VIRTUAL_PATH="${PATH}"
                        if [ -n "${PYTHONHOME+x}" ]; then
                            export _WHI_OLD_VIRTUAL_PYTHONHOME="${PYTHONHOME}"
                            unset PYTHONHOME
                        fi

                        # Store old venv if one was active
                        if [ -n "${VIRTUAL_ENV+x}" ]; then
                            export _WHI_OLD_VIRTUAL_ENV="${VIRTUAL_ENV}"
                        fi

                        # Set new environment
                        export VIRTUAL_ENV="$venv_dir"
                        export PATH="$venv_dir/bin:${PATH}"

                        # Note: VIRTUAL_ENV_PROMPT is already set by whi source, don't override it

                        # Hash reset to ensure commands are found in new PATH
                        if command -v hash >/dev/null 2>&1; then
                            hash -r 2>/dev/null || true
                        fi

                        # Define deactivate function
                        deactivate_pyenv() {
                            # Restore old PATH
                            if [ -n "${_WHI_OLD_VIRTUAL_PATH+x}" ]; then
                                export PATH="${_WHI_OLD_VIRTUAL_PATH}"
                                unset _WHI_OLD_VIRTUAL_PATH
                            fi

                            # Restore PYTHONHOME if it was set
                            if [ -n "${_WHI_OLD_VIRTUAL_PYTHONHOME+x}" ]; then
                                export PYTHONHOME="${_WHI_OLD_VIRTUAL_PYTHONHOME}"
                                unset _WHI_OLD_VIRTUAL_PYTHONHOME
                            fi

                            # Restore old venv if there was one
                            if [ -n "${_WHI_OLD_VIRTUAL_ENV+x}" ]; then
                                export VIRTUAL_ENV="${_WHI_OLD_VIRTUAL_ENV}"
                                unset _WHI_OLD_VIRTUAL_ENV
                            else
                                unset VIRTUAL_ENV
                            fi

                            # Note: Don't unset VIRTUAL_ENV_PROMPT - it belongs to whi source, not pyenv

                            # Hash reset
                            if command -v hash >/dev/null 2>&1; then
                                hash -r 2>/dev/null || true
                            fi

                            # Remove this function
                            unset -f deactivate_pyenv 2>/dev/null || true
                        }
                    fi
                fi
                processed=1
                ;;
            DEACTIVATE_PYENV)
                # Try our custom deactivate function first
                if command -v deactivate_pyenv >/dev/null 2>&1; then
                    deactivate_pyenv
                elif command -v deactivate >/dev/null 2>&1; then
                    # Fall back to standard deactivate (for existing venvs)
                    # Temporarily allow deactivation
                    local prev_allow="${WHI_ALLOW_DEACTIVATE-}"
                    WHI_ALLOW_DEACTIVATE=1
                    deactivate
                    if [ -n "$prev_allow" ]; then
                        WHI_ALLOW_DEACTIVATE="$prev_allow"
                    else
                        unset WHI_ALLOW_DEACTIVATE
                    fi
                fi
                processed=1
                ;;
        esac
    done <<EOF
$output
EOF

    if [ $processed -eq 0 ] && [ -n "$output" ]; then
        __whi_apply_transition_legacy "$output"
    fi

    # Install or remove deactivate guard
    if [ $install_guard -eq 1 ]; then
        __whi_install_deactivate_guard
    fi
    if [ $remove_guard -eq 1 ]; then
        __whi_remove_deactivate_guard
    fi

    return 0
}

__whi_apply_transition_legacy() {
    local input="$1"
    local kind=""
    local var=""
    local value=""
    local line

    while IFS= read -r line; do
        [ -z "$line" ] && continue
        case "$line" in
            kind=*)
                kind=${line#kind=}
                ;;
            var=*)
                var=${line#var=}
                var=${var#\'}
                var=${var%\'}
                ;;
            value=*)
                value=${line#value=}
                value=${value#\'}
                value=${value%\'}
                case "$kind" in
                    PATH)
                        export PATH="$var"
                        ;;
                    SET)
                        if [ -n "$var" ]; then
                            export "$var=$value"
                        fi
                        ;;
                    UNSET)
                        if [ -n "$var" ]; then
                            unset "$var"
                        fi
                        ;;
                esac
                kind=""
                var=""
                value=""
                ;;
        esac
    done <<EOF
$input
EOF

    return 0
}

# Load saved PATH first (if it exists)
# This restores your PATH from the previous session
# Detect shell and load the appropriate saved_path file using whi
if [ -n "$BASH_VERSION" ]; then
    if [ -f ~/.whi/saved_path_bash ]; then
        NEW_PATH=$(__whi_exec __load_saved_path bash 2>/dev/null)
        [ -n "$NEW_PATH" ] && export PATH="$NEW_PATH"
    fi
elif [ -n "$ZSH_VERSION" ]; then
    if [ -f ~/.whi/saved_path_zsh ]; then
        NEW_PATH=$(__whi_exec __load_saved_path zsh 2>/dev/null)
        [ -n "$NEW_PATH" ] && export PATH="$NEW_PATH"
    fi
fi

__whi_apply_path() {
    local subcmd="$1"
    shift
    local new_path
    new_path=$(__whi_exec "__${subcmd}" "$@")
    local exit_code=$?
    if [ $exit_code -eq 0 ]; then
        export PATH="$new_path"
    else
        return $exit_code
    fi
}

__whi_handle_move() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display FROM TO"
            echo "  Move PATH entry from index FROM to index TO"
            return 0
            ;;
    esac

    if [ "$#" -ne 2 ]; then
        echo "Usage: $display FROM TO" >&2
        echo "  Move PATH entry from index FROM to index TO" >&2
        return 2
    fi

    __whi_apply_path move "$1" "$2"
    return $?
}

__whi_handle_switch() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display IDX1 IDX2"
            echo "  Swap PATH entries at indices IDX1 and IDX2"
            return 0
            ;;
    esac

    if [ "$#" -ne 2 ]; then
        echo "Usage: $display IDX1 IDX2" >&2
        echo "  Swap PATH entries at indices IDX1 and IDX2" >&2
        return 2
    fi

    __whi_apply_path switch "$1" "$2"
    return $?
}

__whi_handle_clean() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display"
            echo "  Remove duplicate entries from PATH"
            return 0
            ;;
    esac

    if [ "$#" -ne 0 ]; then
        echo "Usage: $display" >&2
        echo "  Remove duplicate entries from PATH" >&2
        return 2
    fi

    __whi_apply_path clean
    return $?
}

__whi_handle_delete() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display TARGET [TARGET...]"
            echo "  TARGET can be index, path, or fuzzy pattern"
            echo "  Fuzzy patterns delete ALL matching entries"
            return 0
            ;;
    esac

    if [ "$#" -lt 1 ]; then
        echo "Usage: $display TARGET [TARGET...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        return 2
    fi

    __whi_apply_path delete "$@"
    return $?
}

__whi_handle_add() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display PATH..."
            echo "  Add one or more paths to PATH (prepends by default)"
            return 0
            ;;
    esac

    if [ "$#" -lt 1 ]; then
        echo "Usage: $display PATH..." >&2
        return 2
    fi

    __whi_apply_path add "$@"
    return $?
}

__whi_handle_prefer() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display [NAME] TARGET [PATTERN...]"
            echo "  Add path to PATH or prefer executable at target"
            echo "  TARGET can be index, path, or fuzzy pattern"
            printf '  %s ~/.cargo/bin           # Add path to PATH (if not present)\n' "$display"
            printf '  %s cargo 3                # Use cargo from PATH index 3\n' "$display"
            printf '  %s cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo\n' "$display"
            printf '  %s bat github release     # Use bat from path matching pattern\n' "$display"
            return 0
            ;;
    esac

    if [ "$#" -lt 1 ]; then
        echo "Usage: $display [NAME] TARGET [PATTERN...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        return 2
    fi

    if [ "$#" -eq 1 ] && [[ "$1" =~ [/~.] ]]; then
        __whi_apply_path prefer "$1"
    else
        local name="$1"
        shift
        __whi_apply_path prefer "$name" "$@"
    fi
    return $?
}

__whi_handle_redo() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display [COUNT]"
            echo "  Redo next COUNT PATH operations (default: 1)"
            return 0
            ;;
    esac

    if [ "$#" -eq 0 ]; then
        __whi_apply_path redo 1
    elif [ "$#" -eq 1 ]; then
        __whi_apply_path redo "$1"
    else
        echo "Usage: $display [COUNT]" >&2
        echo "  Redo next COUNT PATH operations (default: 1)" >&2
        return 2
    fi
    return $?
}

__whi_handle_undo() {
    local display="$1"
    shift

    case "${1-}" in
        help|--help|-h)
            echo "Usage: $display [COUNT]"
            echo "  Undo last COUNT PATH operations (default: 1)"
            return 0
            ;;
    esac

    if [ "$#" -eq 0 ]; then
        __whi_apply_path undo 1
    elif [ "$#" -eq 1 ]; then
        __whi_apply_path undo "$1"
    else
        echo "Usage: $display [COUNT]" >&2
        echo "  Undo last COUNT PATH operations (default: 1)" >&2
        return 2
    fi
    return $?
}

__whi_venv_source() {
    local dir="${1:-.}"
    __whi_apply_transition __venv_source "$dir"
}

__whi_venv_exit() {
    __whi_apply_transition __venv_exit
}

whim() {
    __whi_handle_move "whim" "$@"
}

whis() {
    __whi_handle_switch "whis" "$@"
}

whip() {
    __whi_handle_prefer "whip" "$@"
}

whic() {
    __whi_handle_clean "whic" "$@"
}

whid() {
    __whi_handle_delete "whid" "$@"
}

whia() {
    __whi_exec --all "$@"
}

whiad() {
    __whi_handle_add "whiad" "$@"
}

whir() {
    __whi_handle_redo "whir" "$@"
}

whiu() {
    __whi_handle_undo "whiu" "$@"
}

whiv() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whiv [-f|--full] [NAME]"
            echo "  Query environment variables"
            echo ""
            echo "Options:"
            echo "  -f, --full    List all environment variables"
            echo ""
            echo "Examples:"
            echo "  whiv PATH         # Show PATH variable"
            echo "  whiv cargo        # Fuzzy search for variables matching 'cargo'"
            echo "  whiv -f           # List all variables"
            return 0
            ;;
    esac
    __whi_exec var "$@"
}

whish() {
    __whi_exec shorthands "$@"
}

whil() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whil NAME"
            echo "  Load saved profile NAME"
            return 0
            ;;
    esac
    if [ "$#" -ne 1 ]; then
        echo "Usage: whil NAME" >&2
        echo "  Load saved profile NAME" >&2
        return 2
    fi
    __whi_apply_transition __load "$1"
}

whi() {
    if [ "$#" -gt 0 ]; then
        case "$1" in
            reset)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi reset"
                        echo "  Reset PATH to initial session state"
                        return 0
                        ;;
                esac
                if [ "$#" -ne 0 ]; then
                    echo "Usage: whi reset" >&2
                    return 2
                fi
                __whi_apply_path reset
                return $?
                ;;
            undo)
                shift
                __whi_handle_undo "whi undo" "$@"
                return $?
                ;;
            redo)
                shift
                __whi_handle_redo "whi redo" "$@"
                return $?
                ;;
            load)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi load NAME"
                        echo "  Load saved profile NAME"
                        return 0
                        ;;
                esac
                if [ "$#" -ne 1 ]; then
                    echo "Usage: whi load NAME" >&2
                    return 2
                fi
                __whi_apply_transition __load "$1"
                return $?
                ;;
            add)
                shift
                __whi_handle_add "whi add" "$@"
                return $?
                ;;
            var)
                shift
                __whi_exec var "$@"
                return $?
                ;;
            shorthands)
                shift
                __whi_exec shorthands "$@"
                return $?
                ;;
            prefer)
                shift
                __whi_handle_prefer "whi prefer" "$@"
                return $?
                ;;
            move)
                shift
                __whi_handle_move "whi move" "$@"
                return $?
                ;;
            switch)
                shift
                __whi_handle_switch "whi switch" "$@"
                return $?
                ;;
            clean)
                shift
                __whi_handle_clean "whi clean" "$@"
                return $?
                ;;
            delete)
                shift
                __whi_handle_delete "whi delete" "$@"
                return $?
                ;;
            source)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi source"
                        echo "  Activate venv from whifile in current directory"
                        return 0
                        ;;
                esac
                if [ "$#" -ne 0 ]; then
                    echo "Usage: whi source" >&2
                    return 2
                fi
                __whi_venv_source "$PWD"
                return $?
                ;;
            exit)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi exit"
                        echo "  Exit active venv and restore previous PATH"
                        return 0
                        ;;
                esac
                if [ "$#" -ne 0 ]; then
                    echo "Usage: whi exit" >&2
                    return 2
                fi
                __whi_venv_exit
                return $?
                ;;
        esac
    fi

    __whi_exec "$@"
}

__whi_prompt() {
    if [ -n "${VIRTUAL_ENV_DISABLE_PROMPT-}" ]; then
        return
    fi

    if [ -n "${_OLD_VIRTUAL_PS1+set}" ]; then
        # Venv was sourced manually and is already managing PS1 – avoid duplicating the prefix.
        return
    fi

    local prompt="${VIRTUAL_ENV_PROMPT-}"
    if [ -n "$prompt" ]; then
        # Check if VIRTUAL_ENV_PROMPT is already formatted with parens (from Python venv)
        # or just a name (from whi)
        case "$prompt" in
            '('*)
                # Already formatted by Python's activate, use as-is
                printf '%s' "$prompt"
                ;;
            *)
                # From whi, needs formatting
                printf '(%s) ' "$prompt"
                ;;
        esac
    elif [ -n "${VIRTUAL_ENV-}" ]; then
        printf '(%s) ' "${VIRTUAL_ENV##*/}"
    fi
}
if [ -n "$BASH_VERSION" ]; then
    __whi_add_prompt_command() {
        local fn="$1"
        local decl
        decl=$(declare -p PROMPT_COMMAND 2>/dev/null || printf '')
        case "$decl" in
            declare\ -a*)
                case " ${PROMPT_COMMAND[*]} " in *" $fn "*) ;; *) PROMPT_COMMAND+=("$fn") ;; esac
                ;;
            *)
                if [ -n "${PROMPT_COMMAND:-}" ]; then
                    case "$PROMPT_COMMAND" in
                        *$'\n'"$fn"$'\n'*|*$'\n'"$fn"|"$fn"$'\n'*|"$fn") ;;
                        *) PROMPT_COMMAND="${PROMPT_COMMAND}"$'\n'"$fn" ;;
                    esac
                else
                    PROMPT_COMMAND="$fn"
                fi
                ;;
        esac
    }

    if [ -z "${__WHI_BASH_CD_INSTALLED:-}" ]; then
        __WHI_BASH_CD_INSTALLED=1
        __whi_add_prompt_command __whi_cd_hook
    fi

    if [ -z "${__WHI_BASH_PROMPT_INSTALLED:-}" ]; then
        __WHI_BASH_PROMPT_INSTALLED=1
        __whi_prompt_command() {
            local last_status=$?
            local prefix="$(__whi_prompt)"
            local current="${PS1-}"
            if [ "${__WHI_BASH_LAST_PROMPT:-}" != "$current" ]; then
                __WHI_BASH_BASE_PROMPT="$current"
            fi
            if [ -z "${__WHI_BASH_BASE_PROMPT+x}" ]; then
                __WHI_BASH_BASE_PROMPT="$current"
            fi
            if [ -n "$prefix" ]; then
                PS1="${prefix}${__WHI_BASH_BASE_PROMPT}"
            else
                PS1="${__WHI_BASH_BASE_PROMPT}"
            fi
            __WHI_BASH_LAST_PROMPT="$PS1"
            return $last_status
        }
        __whi_add_prompt_command __whi_prompt_command
    fi
elif [ -n "$ZSH_VERSION" ]; then
    if [ -z "${__WHI_ZSH_CD_INSTALLED:-}" ]; then
        __WHI_ZSH_CD_INSTALLED=1
        autoload -Uz add-zsh-hook 2>/dev/null || true

        # Register (append) then move to front so failures in other hooks can't block us
        if ! (( ${chpwd_functions[(Ie)__whi_cd_hook]} )); then
            add-zsh-hook chpwd __whi_cd_hook 2>/dev/null || chpwd_functions+=(__whi_cd_hook)
        fi
        # Prepend ours (keeping order of others)
        chpwd_functions=( __whi_cd_hook ${chpwd_functions:#__whi_cd_hook} )
    fi

    if [ -z "${__WHI_ZSH_PROMPT_INSTALLED:-}" ]; then
        __WHI_ZSH_PROMPT_INSTALLED=1
        setopt prompt_subst 2>/dev/null
        autoload -Uz add-zsh-hook 2>/dev/null || true

        __whi_precmd_prompt() {
            local last_status=$?
            typeset -g WHI_PROMPT_PREFIX
            WHI_PROMPT_PREFIX="$(__whi_prompt)"
            WHI_PROMPT_PREFIX=${WHI_PROMPT_PREFIX//%/%%}

            if [[ -n ${__WHI_ZSH_LAST_PROMPT-} ]] && [[ "$PROMPT" != "${__WHI_ZSH_LAST_PROMPT}" ]]; then
                typeset -g __WHI_ZSH_BASE_PROMPT="$PROMPT"
            elif [[ -z ${__WHI_ZSH_BASE_PROMPT+x} ]]; then
                typeset -g __WHI_ZSH_BASE_PROMPT="$PROMPT"
            fi

            if [[ -n "$WHI_PROMPT_PREFIX" ]]; then
                PROMPT="${WHI_PROMPT_PREFIX}${__WHI_ZSH_BASE_PROMPT}"
            else
                PROMPT="${__WHI_ZSH_BASE_PROMPT}"
            fi

            typeset -g __WHI_ZSH_LAST_PROMPT="$PROMPT"
            return $last_status
        }

        # Lightweight fallback: if some tool changed $PWD without firing chpwd, call our hook
        __whi_precmd_cd_guard() {
            local cur="${PWD:-}"
            if [[ "${__WHI_LAST_PWD-}" != "$cur" ]]; then
                __whi_cd_hook   # idempotent: it exits fast if nothing to do
            fi
            return 0    # Always return 0 to avoid blocking subsequent hooks
        }

        # Register once
        if ! (( ${precmd_functions[(Ie)__whi_precmd_prompt]} )); then
            add-zsh-hook precmd __whi_precmd_prompt 2>/dev/null || precmd_functions+=(__whi_precmd_prompt)
        fi
        if ! (( ${precmd_functions[(Ie)__whi_precmd_cd_guard]} )); then
            add-zsh-hook precmd __whi_precmd_cd_guard 2>/dev/null || precmd_functions+=(__whi_precmd_cd_guard)
        fi

        __whi_precmd_prompt
    fi
fi

__whi_cd_hook() {
    local current_pwd="${PWD:-}"
    local last_pwd="${__WHI_LAST_PWD-}"
    local pwd_changed=1
    if [ -n "$last_pwd" ] && [ "$current_pwd" = "$last_pwd" ]; then
        pwd_changed=0
    fi

    if [ -n "$__WHI_CONFIG_PATH" ]; then
        local current_mtime
        current_mtime=$(__whi_stat_mtime "$__WHI_CONFIG_PATH")
        if [ -z "$current_mtime" ]; then
            if [ -n "${__WHI_AUTO_FILE_MTIME-}" ]; then
                __WHI_AUTO_FILE=0
                __WHI_AUTO_FILE_MTIME=""
            fi
        elif [ "$current_mtime" != "${__WHI_AUTO_FILE_MTIME-}" ]; then
            __whi_refresh_auto_config
        fi
    fi

    local has_file=0
    [ -f "$current_pwd/whifile" ] && has_file=1

    local file_changed=1
    if [ -n "${__WHI_LAST_HAS_FILE+x}" ] && [ "${__WHI_LAST_HAS_FILE:-0}" -eq "$has_file" ]; then
        file_changed=0
    fi

    if [ $pwd_changed -eq 0 ] && [ $file_changed -eq 0 ]; then
        __WHI_LAST_PWD="$current_pwd"
        __WHI_LAST_HAS_FILE=$has_file
        return 0
    fi

    if [ -n "$WHI_VENV_DIR" ] && [ "${__WHI_AUTO_DEACTIVATE:-0}" -eq 1 ] && [ $pwd_changed -eq 1 ]; then
        case "${current_pwd%/}/" in
            "${WHI_VENV_DIR%/}/" | "${WHI_VENV_DIR%/}/"*)
                ;;
            *)
                __whi_venv_exit 2>/dev/null
                ;;
        esac
    fi

    if [ -z "${VIRTUAL_ENV-}" ] && [ "${__WHI_AUTO_FILE:-0}" -eq 1 ] && [ $has_file -eq 1 ]; then
        if [ $pwd_changed -eq 1 ] || [ $file_changed -eq 1 ]; then
            __whi_venv_source "$current_pwd" 2>/dev/null
        fi
    fi

    __WHI_LAST_PWD="$current_pwd"
    __WHI_LAST_HAS_FILE=$has_file
    return 0    # Always return 0 to avoid blocking subsequent hooks
}

# Hook cd to check for venv auto-activation (bash only - zsh uses chpwd hook above)
if [ -n "$BASH_VERSION" ]; then
    __whi_cd() {
        builtin cd "$@" && __whi_cd_hook
    }
    alias cd='__whi_cd'
fi

if [ -z "$WHI_SHELL_INITIALIZED" ]; then
    export WHI_SHELL_INITIALIZED=1
    export WHI_SESSION_PID=$$
    __whi_exec __init "$WHI_SESSION_PID" 2>/dev/null
fi

# Trigger auto-activation for the current directory (if configured)
__whi_cd_hook >/dev/null 2>&1 || true

# IMPORTANT: Add this to the END of your shell config:
#
#   bash: Add to ~/.bashrc:  eval "$(whi init bash)"
#   zsh:  Add to ~/.zshrc:   eval "$(whi init zsh)"
#
# This must be at the END so whi captures your final PATH after all modifications.
#
# Also remove any old "# whi: Load saved PATH" sections from your config -
# saved PATH loading is now included at the top of this integration script.
#
# Or run in the current shell:
#
#   bash: eval "$(whi init bash)"
#   zsh:  eval "$(whi init zsh)"
