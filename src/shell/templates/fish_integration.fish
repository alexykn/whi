# whi shell integration for fish (path-only)

set -gx __WHI_BIN "__WHI_BIN__"

function __whi_run
    $__WHI_BIN $argv
end

function __whi_apply
    set -l subcmd $argv[1]
    set -l rest $argv[2..-1]
    set -l new_path (__whi_run __$subcmd $rest)
    set -l exit_code $status
    if test $exit_code -ne 0
        return $exit_code
    end

    set -gx PATH (string split : -- $new_path)
end

function __whi_handle_move --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ne 2
        echo "Usage: $display FROM TO" >&2
        return 2
    end
    __whi_apply move $args
end

function __whi_handle_switch --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ne 2
        echo "Usage: $display IDX1 IDX2" >&2
        return 2
    end
    __whi_apply switch $args
end

function __whi_handle_clean --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ne 0
        echo "Usage: $display" >&2
        return 2
    end
    __whi_apply clean
end

function __whi_handle_delete --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -lt 1
        echo "Usage: $display TARGET [TARGET...]" >&2
        return 2
    end
    __whi_apply delete $args
end

function __whi_handle_add --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -lt 1
        echo "Usage: $display PATH..." >&2
        return 2
    end
    __whi_apply add $args
end

function __whi_handle_prefer --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -lt 1
        echo "Usage: $display [NAME] TARGET [PATTERN...]" >&2
        return 2
    end

    if test (count $args) -eq 1 -a (string match -qr '[/~.]' -- $args[1])
        __whi_apply prefer $args
    else
        set -l name $args[1]
        set -l rest $args[2..-1]
        __whi_apply prefer $name $rest
    end
end

function __whi_handle_redo --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -gt 1
        echo "Usage: $display [COUNT]" >&2
        return 2
    end

    if test (count $args) -eq 0
        __whi_apply redo 1
    else
        __whi_apply redo $args[1]
    end
end

function __whi_handle_undo --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -gt 1
        echo "Usage: $display [COUNT]" >&2
        return 2
    end

    if test (count $args) -eq 0
        __whi_apply undo 1
    else
        __whi_apply undo $args[1]
    end
end

if test -f ~/.whi/saved_path_fish
    set -l new_path (__whi_run __load_saved_path fish 2>/dev/null)
    if test -n "$new_path"
        set -gx PATH (string split : -- $new_path)
    end
end

function whim
    __whi_handle_move whim $argv
end

function whis
    __whi_handle_switch whis $argv
end

function whip
    __whi_handle_prefer whip $argv
end

function whic
    __whi_handle_clean whic $argv
end

function whid
    __whi_handle_delete whid $argv
end

function whia
    __whi_run --all $argv
end

function whiad
    __whi_handle_add whiad $argv
end

function whir
    __whi_handle_redo whir $argv
end

function whiu
    __whi_handle_undo whiu $argv
end

function whil
    if test (count $argv) -ne 1
        echo "Usage: whil NAME" >&2
        return 2
    end
    __whi_apply load $argv[1]
end

function whish
    __whi_run shorthands $argv
end

function whi
    if test (count $argv) -eq 0
        __whi_run
        return $status
    end

    set -l cmd $argv[1]
    set -l rest $argv[2..-1]

    switch $cmd
        case reset
            if test (count $rest) -ne 0
                echo "Usage: whi reset" >&2
                return 2
            end
            __whi_apply reset
        case undo
            __whi_handle_undo "whi undo" $rest
        case redo
            __whi_handle_redo "whi redo" $rest
        case load
            if test (count $rest) -ne 1
                echo "Usage: whi load NAME" >&2
                return 2
            end
            __whi_apply load $rest[1]
        case add
            __whi_handle_add "whi add" $rest
        case prefer
            __whi_handle_prefer "whi prefer" $rest
        case move
            __whi_handle_move "whi move" $rest
        case switch
            __whi_handle_switch "whi switch" $rest
        case clean
            __whi_handle_clean "whi clean" $rest
        case delete
            __whi_handle_delete "whi delete" $rest
        case '*'
            __whi_run $argv
    end
end

set -gx WHI_SHELL_INITIALIZED 1
if not set -q WHI_SESSION_PID
    set -gx WHI_SESSION_PID %self
end
__whi_run __init $WHI_SESSION_PID >/dev/null 2>&1
