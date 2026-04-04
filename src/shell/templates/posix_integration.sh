# whi shell integration for bash/zsh (path-only)

__WHI_BIN="__WHI_BIN__"

__whi_exec() {
    "$__WHI_BIN" "$@"
}

__whi_apply_path() {
    local subcmd="$1"
    shift

    local new_path
    new_path=$(__whi_exec "__${subcmd}" "$@")
    local exit_code=$?
    if [ $exit_code -ne 0 ]; then
        return $exit_code
    fi

    export PATH="$new_path"
    hash -r 2>/dev/null || true
}

__whi_handle_move() {
    local display="$1"
    shift
    [ "$#" -eq 2 ] || {
        echo "Usage: $display FROM TO" >&2
        return 2
    }
    __whi_apply_path move "$1" "$2"
}

__whi_handle_switch() {
    local display="$1"
    shift
    [ "$#" -eq 2 ] || {
        echo "Usage: $display IDX1 IDX2" >&2
        return 2
    }
    __whi_apply_path switch "$1" "$2"
}

__whi_handle_clean() {
    local display="$1"
    shift
    [ "$#" -eq 0 ] || {
        echo "Usage: $display" >&2
        return 2
    }
    __whi_apply_path clean
}

__whi_handle_delete() {
    local display="$1"
    shift
    [ "$#" -ge 1 ] || {
        echo "Usage: $display TARGET [TARGET...]" >&2
        return 2
    }
    __whi_apply_path delete "$@"
}

__whi_handle_add() {
    local display="$1"
    shift
    [ "$#" -ge 1 ] || {
        echo "Usage: $display PATH..." >&2
        return 2
    }
    __whi_apply_path add "$@"
}

__whi_handle_prefer() {
    local display="$1"
    shift
    [ "$#" -ge 1 ] || {
        echo "Usage: $display [NAME] TARGET [PATTERN...]" >&2
        return 2
    }

    if [ "$#" -eq 1 ] && [[ "$1" =~ [/~.] ]]; then
        __whi_apply_path prefer "$1"
    else
        local name="$1"
        shift
        __whi_apply_path prefer "$name" "$@"
    fi
}

__whi_handle_redo() {
    local display="$1"
    shift
    [ "$#" -le 1 ] || {
        echo "Usage: $display [COUNT]" >&2
        return 2
    }
    if [ "$#" -eq 0 ]; then
        __whi_apply_path redo 1
    else
        __whi_apply_path redo "$1"
    fi
}

__whi_handle_undo() {
    local display="$1"
    shift
    [ "$#" -le 1 ] || {
        echo "Usage: $display [COUNT]" >&2
        return 2
    }
    if [ "$#" -eq 0 ]; then
        __whi_apply_path undo 1
    else
        __whi_apply_path undo "$1"
    fi
}

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

whim() { __whi_handle_move "whim" "$@"; }
whis() { __whi_handle_switch "whis" "$@"; }
whip() { __whi_handle_prefer "whip" "$@"; }
whic() { __whi_handle_clean "whic" "$@"; }
whid() { __whi_handle_delete "whid" "$@"; }
whia() { __whi_exec --all "$@"; }
whiad() { __whi_handle_add "whiad" "$@"; }
whin() { __whi_exec -n "$@"; }
whir() { __whi_handle_redo "whir" "$@"; }
whiu() { __whi_handle_undo "whiu" "$@"; }
whil() {
    [ "$#" -eq 1 ] || {
        echo "Usage: whil NAME" >&2
        return 2
    }
    __whi_apply_path load "$1"
}
whish() { __whi_exec shorthands "$@"; }

whi() {
    if [ "$#" -eq 0 ]; then
        __whi_exec
        return $?
    fi

    local cmd="$1"
    shift

    case "$cmd" in
        reset)
            [ "$#" -eq 0 ] || {
                echo "Usage: whi reset" >&2
                return 2
            }
            __whi_apply_path reset
            ;;
        undo)
            __whi_handle_undo "whi undo" "$@"
            ;;
        redo)
            __whi_handle_redo "whi redo" "$@"
            ;;
        load)
            [ "$#" -eq 1 ] || {
                echo "Usage: whi load NAME" >&2
                return 2
            }
            __whi_apply_path load "$1"
            ;;
        add)
            __whi_handle_add "whi add" "$@"
            ;;
        prefer)
            __whi_handle_prefer "whi prefer" "$@"
            ;;
        move)
            __whi_handle_move "whi move" "$@"
            ;;
        switch)
            __whi_handle_switch "whi switch" "$@"
            ;;
        clean)
            __whi_handle_clean "whi clean" "$@"
            ;;
        delete)
            __whi_handle_delete "whi delete" "$@"
            ;;
        *)
            __whi_exec "$cmd" "$@"
            ;;
    esac
}

export WHI_SHELL_INITIALIZED=1
export WHI_SESSION_PID="${WHI_SESSION_PID:-$$}"
__whi_exec __init "$WHI_SESSION_PID" >/dev/null 2>&1 || true
