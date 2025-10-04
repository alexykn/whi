# whi shell integration for bash/zsh (v0.5.2)

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
    local output
    output=$(__whi_exec __should_auto_activate 2>/dev/null || printf 'file=0')
    output=${output%%$'\n'*}

    case "$output" in
        file=1) __WHI_AUTO_FILE=1 ;;
        file=0) __WHI_AUTO_FILE=0 ;;
        *) __WHI_AUTO_FILE=0 ;;
    esac

    if [ -n "$__WHI_CONFIG_PATH" ]; then
        local mtime
        mtime=$(__whi_stat_mtime "$__WHI_CONFIG_PATH")
        __WHI_AUTO_FILE_MTIME="$mtime"
    fi
}

__whi_apply_transition() {
    local output
    if ! output="$(__whi_exec "$@")"; then
        return $?
    fi

    local tab=$'\t'
    local processed=0
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
                fi
                processed=1
                ;;
            UNSET"$tab"*)
                var=${line#UNSET$tab}
                if [ -n "$var" ]; then
                    unset "$var"
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

__whi_venv_source() {
    local dir="${1:-.}"
    __whi_apply_transition __venv_source "$dir"
}

__whi_venv_exit() {
    __whi_apply_transition __venv_exit
}

__whi_refresh_auto_config

whim() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whim FROM TO"
            echo "  Move PATH entry from index FROM to index TO"
            return 0
            ;;
    esac
    if [ "$#" -ne 2 ]; then
        echo "Usage: whim FROM TO" >&2
        echo "  Move PATH entry from index FROM to index TO" >&2
        return 2
    fi

    __whi_apply_path move "$1" "$2"
}

whis() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whis IDX1 IDX2"
            echo "  Swap PATH entries at indices IDX1 and IDX2"
            return 0
            ;;
    esac
    if [ "$#" -ne 2 ]; then
        echo "Usage: whis IDX1 IDX2" >&2
        echo "  Swap PATH entries at indices IDX1 and IDX2" >&2
        return 2
    fi

    __whi_apply_path switch "$1" "$2"
}

whip() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whip [NAME] TARGET [PATTERN...]"
            echo "  Add path to PATH or prefer executable at target"
            echo "  TARGET can be index, path, or fuzzy pattern"
            echo "Examples:"
            echo "  whip ~/.cargo/bin           # Add path to PATH (if not present)"
            echo "  whip cargo 3                # Use cargo from PATH index 3"
            echo "  whip cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo"
            echo "  whip bat github release     # Use bat from path matching pattern"
            return 0
            ;;
    esac
    if [ "$#" -lt 1 ]; then
        echo "Usage: whip [NAME] TARGET [PATTERN...]" >&2
        echo "  Add path to PATH or prefer executable at target" >&2
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
}

whic() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whic"
            echo "  Remove duplicate entries from PATH"
            return 0
            ;;
    esac
    if [ "$#" -ne 0 ]; then
        echo "Usage: whic" >&2
        echo "  Remove duplicate entries from PATH" >&2
        return 2
    fi
    __whi_apply_path clean
}

whid() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whid TARGET [TARGET...]"
            echo "  TARGET can be index, path, or fuzzy pattern"
            echo "  Fuzzy patterns delete ALL matching entries"
            echo "Examples:"
            echo "  whid 3                      # Delete PATH entry at index 3"
            echo "  whid 2 5 7                  # Delete multiple indices"
            echo "  whid ~/.local/bin           # Delete ~/.local/bin from PATH"
            echo "  whid temp bin               # Delete ALL entries matching pattern"
            return 0
            ;;
    esac
    if [ "$#" -lt 1 ]; then
        echo "Usage: whid TARGET [TARGET...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        return 2
    fi

    __whi_apply_path delete "$@"
}

whia() {
    __whi_exec --all "$@"
}

whir() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whir [COUNT]"
            echo "  Redo next COUNT PATH operations (default: 1)"
            return 0
            ;;
    esac
    if [ "$#" -eq 0 ]; then
        __whi_apply_path redo 1
    elif [ "$#" -eq 1 ]; then
        __whi_apply_path redo "$1"
    else
        echo "Usage: whir [COUNT]" >&2
        echo "  Redo next COUNT PATH operations (default: 1)" >&2
        return 2
    fi
}

whiu() {
    case "$1" in
        help|--help|-h)
            echo "Usage: whiu [COUNT]"
            echo "  Undo last COUNT PATH operations (default: 1)"
            return 0
            ;;
    esac
    if [ "$#" -eq 0 ]; then
        __whi_apply_path undo 1
    elif [ "$#" -eq 1 ]; then
        __whi_apply_path undo "$1"
    else
        echo "Usage: whiu [COUNT]" >&2
        echo "  Undo last COUNT PATH operations (default: 1)" >&2
        return 2
    fi
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
    __whi_apply_path load "$1"
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
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi undo [COUNT]"
                        echo "  Undo last COUNT PATH operations (default: 1)"
                        return 0
                        ;;
                esac
                if [ "$#" -eq 0 ]; then
                    __whi_apply_path undo 1
                elif [ "$#" -eq 1 ]; then
                    __whi_apply_path undo "$1"
                else
                    echo "Usage: whi undo [COUNT]" >&2
                    return 2
                fi
                return $?
                ;;
            redo)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi redo [COUNT]"
                        echo "  Redo next COUNT PATH operations (default: 1)"
                        return 0
                        ;;
                esac
                if [ "$#" -eq 0 ]; then
                    __whi_apply_path redo 1
                elif [ "$#" -eq 1 ]; then
                    __whi_apply_path redo "$1"
                else
                    echo "Usage: whi redo [COUNT]" >&2
                    return 2
                fi
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
                __whi_apply_path load "$1"
                return $?
                ;;
            prefer)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi prefer [NAME] TARGET [PATTERN...]"
                        echo "  Add path to PATH or prefer executable at target"
                        echo "  TARGET can be index, path, or fuzzy pattern"
                        return 0
                        ;;
                esac
                if [ "$#" -lt 1 ]; then
                    echo "Usage: whi prefer [NAME] TARGET [PATTERN...]" >&2
                    return 2
                fi
                __whi_apply_path prefer "$@"
                return $?
                ;;
            move)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi move FROM TO"
                        echo "  Move PATH entry from index FROM to index TO"
                        return 0
                        ;;
                esac
                if [ "$#" -ne 2 ]; then
                    echo "Usage: whi move FROM TO" >&2
                    return 2
                fi
                __whi_apply_path move "$@"
                return $?
                ;;
            switch)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi switch IDX1 IDX2"
                        echo "  Swap PATH entries at indices IDX1 and IDX2"
                        return 0
                        ;;
                esac
                if [ "$#" -ne 2 ]; then
                    echo "Usage: whi switch IDX1 IDX2" >&2
                    return 2
                fi
                __whi_apply_path switch "$@"
                return $?
                ;;
            clean)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi clean"
                        echo "  Remove duplicate entries from PATH"
                        return 0
                        ;;
                esac
                if [ "$#" -ne 0 ]; then
                    echo "Usage: whi clean" >&2
                    return 2
                fi
                __whi_apply_path clean "$@"
                return $?
                ;;
            delete)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi delete TARGET [TARGET...]"
                        echo "  TARGET can be index, path, or fuzzy pattern"
                        return 0
                        ;;
                esac
                if [ "$#" -lt 1 ]; then
                    echo "Usage: whi delete TARGET [TARGET...]" >&2
                    return 2
                fi
                __whi_apply_path delete "$@"
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
    if [ -n "$WHI_VENV_NAME" ]; then
        echo "[${WHI_VENV_NAME}] "
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

    if command -v starship >/dev/null 2>&1 && [[ "${PROMPT_COMMAND:-}" == *starship* ]]; then
        :  # prompt handled entirely by starship configuration
    elif command -v oh-my-posh >/dev/null 2>&1 && [[ "${PROMPT_COMMAND:-}" == *oh-my-posh* ]]; then
        :  # oh-my-posh reads env vars on its own
    else
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
            PS1="${prefix}${__WHI_BASH_BASE_PROMPT}"
            __WHI_BASH_LAST_PROMPT="$PS1"
            return $last_status
        }

        if [ -z "${__WHI_BASH_PROMPT_INSTALLED:-}" ]; then
            __WHI_BASH_PROMPT_INSTALLED=1
            __whi_add_prompt_command __whi_prompt_command
        fi
    fi
elif [ -n "$ZSH_VERSION" ]; then
    if command -v starship >/dev/null 2>&1 && [[ "$(typeset -f precmd 2>/dev/null)" == *starship* ]]; then
        :  # starship users configure the indicator themselves
    elif command -v oh-my-posh >/dev/null 2>&1 && [[ "$(typeset -f precmd 2>/dev/null)" == *oh-my-posh* ]]; then
        :  # oh-my-posh reads env vars on its own
    elif [[ -n "${POWERLEVEL9K_VERSION:-}${POWERLEVEL10K_VERSION:-}" ]] || [[ "${ZSH_THEME:-}" == *powerlevel* ]]; then
        typeset -g POWERLEVEL9K_WHI_VENV_FOREGROUND=yellow
        prompt_whi_venv() {
            local venv_text="$(__whi_prompt)"
            if [[ -n "$venv_text" ]]; then
                p10k segment -f $POWERLEVEL9K_WHI_VENV_FOREGROUND -t "$venv_text"
            fi
        }
    else
        __whi_precmd_prompt() {
            local last_status=$?
            typeset -g WHI_PROMPT_PREFIX
            WHI_PROMPT_PREFIX="$(__whi_prompt)"
            WHI_PROMPT_PREFIX=${WHI_PROMPT_PREFIX//\%/%%}

            if [[ -n ${__WHI_ZSH_LAST_PROMPT-} ]] && [[ "$PROMPT" != "${__WHI_ZSH_LAST_PROMPT}" ]]; then
                typeset -g __WHI_ZSH_BASE_PROMPT="$PROMPT"
            elif [[ -z ${__WHI_ZSH_BASE_PROMPT+x} ]]; then
                typeset -g __WHI_ZSH_BASE_PROMPT="$PROMPT"
            fi

            PROMPT="${WHI_PROMPT_PREFIX}${__WHI_ZSH_BASE_PROMPT}"
            typeset -g __WHI_ZSH_LAST_PROMPT="$PROMPT"
            return $last_status
        }

        if [ -z "${__WHI_ZSH_PROMPT_INSTALLED:-}" ]; then
            __WHI_ZSH_PROMPT_INSTALLED=1
            setopt prompt_subst 2>/dev/null
            autoload -Uz add-zsh-hook 2>/dev/null
            if typeset -f add-zsh-hook >/dev/null 2>&1; then
                add-zsh-hook precmd __whi_precmd_prompt 2>/dev/null || case " ${precmd_functions[*]:-} " in *" __whi_precmd_prompt "*) ;; *) precmd_functions+=(__whi_precmd_prompt) ;; esac
            else
                typeset -ga precmd_functions 2>/dev/null
                case " ${precmd_functions[*]:-} " in *" __whi_precmd_prompt "*) ;; *) precmd_functions+=(__whi_precmd_prompt) ;; esac
            fi
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

    if [ -n "$WHI_VENV_DIR" ] && [ $pwd_changed -eq 1 ]; then
        case "${current_pwd%/}/" in
            "${WHI_VENV_DIR%/}/" | "${WHI_VENV_DIR%/}/"*)
                ;;
            *)
                __whi_venv_exit 2>/dev/null
                ;;
        esac
    fi

    if [ -z "$WHI_VENV_NAME" ] && [ "${__WHI_AUTO_FILE:-0}" -eq 1 ] && [ $has_file -eq 1 ]; then
        if [ $pwd_changed -eq 1 ] || [ $file_changed -eq 1 ]; then
            __whi_venv_source "$current_pwd" 2>/dev/null
        fi
    fi

    __WHI_LAST_PWD="$current_pwd"
    __WHI_LAST_HAS_FILE=$has_file
}

# Hook cd to check for venv auto-activation
if [ -n "$BASH_VERSION" ]; then
    __whi_cd() {
        builtin cd "$@" && __whi_cd_hook
    }
    alias cd='__whi_cd'
elif [ -n "$ZSH_VERSION" ]; then
    chpwd_functions+=(__whi_cd_hook)
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
