# whi shell integration for bash/zsh (v0.4.1)

# Load saved PATH first (if it exists)
# Detect shell and load the appropriate saved_path file
if [ -n "$BASH_VERSION" ]; then
    [ -f ~/.whi/saved_path_bash ] && export PATH="$(cat ~/.whi/saved_path_bash)"
elif [ -n "$ZSH_VERSION" ]; then
    [ -f ~/.whi/saved_path_zsh ] && export PATH="$(cat ~/.whi/saved_path_zsh)"
fi

__whi_apply_path() {
    local subcmd="$1"
    shift
    local new_path
    new_path=$(command whi "__${subcmd}" "$@")
    local exit_code=$?
    if [ $exit_code -eq 0 ]; then
        export PATH="$new_path"
    else
        return $exit_code
    fi
}

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
    command whi --all "$@"
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
        esac
    fi

    command whi "$@"
}

if [ -z "$WHI_SHELL_INITIALIZED" ]; then
    export WHI_SHELL_INITIALIZED=1
    export WHI_SESSION_PID=$$
    command whi __init "$WHI_SESSION_PID" 2>/dev/null
fi

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
