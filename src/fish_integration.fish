# whi shell integration for fish (v0.4.0)

# Load saved PATH first (if it exists)
if test -f ~/.whi/saved_path_fish
    set -gx PATH (cat ~/.whi/saved_path_fish | string split :)
end

function __whi_apply
    set -l subcmd $argv[1]
    set -l rest $argv[2..-1]
    set -l new_path (command whi __$subcmd $rest)
    set -l exit_code $status
    if test $exit_code -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $exit_code
    end
end

function whim
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whim FROM TO"
        echo "  Move PATH entry from index FROM to index TO"
        return 0
    end
    if test (count $argv) -ne 2
        echo "Usage: whim FROM TO" >&2
        return 2
    end

    __whi_apply move $argv
end

function whis
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whis IDX1 IDX2"
        echo "  Swap PATH entries at indices IDX1 and IDX2"
        return 0
    end
    if test (count $argv) -ne 2
        echo "Usage: whis IDX1 IDX2" >&2
        return 2
    end

    __whi_apply switch $argv
end

function whip
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whip [NAME] TARGET [PATTERN...]"
        echo "  Add path to PATH or prefer executable at target"
        return 0
    end
    if test (count $argv) -lt 1
        echo "Usage: whip [NAME] TARGET [PATTERN...]" >&2
        return 2
    end

    if test (count $argv) -eq 1 -a (string match -qr '[/~.]' -- $argv[1])
        __whi_apply prefer $argv
    else
        set -l name $argv[1]
        __whi_apply prefer $name $argv[2..-1]
    end
end

function whic
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whic"
        echo "  Remove duplicate entries from PATH"
        return 0
    end
    if test (count $argv) -ne 0
        echo "Usage: whic" >&2
        return 2
    end
    __whi_apply clean
end

function whid
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whid TARGET [TARGET...]"
        echo "  TARGET can be index, path, or fuzzy pattern"
        return 0
    end
    if test (count $argv) -lt 1
        echo "Usage: whid TARGET [TARGET...]" >&2
        return 2
    end

    __whi_apply delete $argv
end

function whia
    command whi --all $argv
end

function whir
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whir [COUNT]"
        echo "  Redo next COUNT PATH operations (default: 1)"
        return 0
    end
    if test (count $argv) -eq 0
        __whi_apply redo 1
    else if test (count $argv) -eq 1
        __whi_apply redo $argv[1]
    else
        echo "Usage: whir [COUNT]" >&2
        return 2
    end
end

function whiu
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whiu [COUNT]"
        echo "  Undo last COUNT PATH operations (default: 1)"
        return 0
    end
    if test (count $argv) -eq 0
        __whi_apply undo 1
    else if test (count $argv) -eq 1
        __whi_apply undo $argv[1]
    else
        echo "Usage: whiu [COUNT]" >&2
        return 2
    end
end

function whil
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whil NAME"
        echo "  Load saved profile NAME"
        return 0
    end
    if test (count $argv) -ne 1
        echo "Usage: whil NAME" >&2
        return 2
    end
    __whi_apply load $argv[1]
end

function whi
    if test (count $argv) -gt 0
        switch $argv[1]
            case reset
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi reset"
                    echo "  Reset PATH to initial session state"
                    return 0
                end
                if test (count $argv) -ne 1
                    echo "Usage: whi reset" >&2
                    return 2
                end
                __whi_apply reset
                return $status
            case undo
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi undo [COUNT]"
                    echo "  Undo last COUNT PATH operations (default: 1)"
                    return 0
                end
                if test (count $argv) -eq 1
                    __whi_apply undo 1
                else if test (count $argv) -eq 2
                    __whi_apply undo $argv[2]
                else
                    echo "Usage: whi undo [COUNT]" >&2
                    return 2
                end
                return $status
            case redo
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi redo [COUNT]"
                    echo "  Redo next COUNT PATH operations (default: 1)"
                    return 0
                end
                if test (count $argv) -eq 1
                    __whi_apply redo 1
                else if test (count $argv) -eq 2
                    __whi_apply redo $argv[2]
                else
                    echo "Usage: whi redo [COUNT]" >&2
                    return 2
                end
                return $status
            case load
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi load NAME"
                    echo "  Load saved profile NAME"
                    return 0
                end
                if test (count $argv) -ne 2
                    echo "Usage: whi load NAME" >&2
                    return 2
                end
                __whi_apply load $argv[2]
                return $status
            case prefer
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi prefer [NAME] TARGET [PATTERN...]"
                    echo "  Add path to PATH or prefer executable at target"
                    return 0
                end
                if test (count $argv) -lt 2
                    echo "Usage: whi prefer [NAME] TARGET [PATTERN...]" >&2
                    return 2
                end
                __whi_apply prefer $argv[2..-1]
                return $status
            case move
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi move FROM TO"
                    echo "  Move PATH entry from index FROM to index TO"
                    return 0
                end
                if test (count $argv) -ne 3
                    echo "Usage: whi move FROM TO" >&2
                    return 2
                end
                __whi_apply move $argv[2..-1]
                return $status
            case switch
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi switch IDX1 IDX2"
                    echo "  Swap PATH entries at indices IDX1 and IDX2"
                    return 0
                end
                if test (count $argv) -ne 3
                    echo "Usage: whi switch IDX1 IDX2" >&2
                    return 2
                end
                __whi_apply switch $argv[2..-1]
                return $status
            case clean
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi clean"
                    echo "  Remove duplicate entries from PATH"
                    return 0
                end
                if test (count $argv) -ne 1
                    echo "Usage: whi clean" >&2
                    return 2
                end
                __whi_apply clean
                return $status
            case delete
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi delete TARGET [TARGET...]"
                    echo "  TARGET can be index, path, or fuzzy pattern"
                    return 0
                end
                if test (count $argv) -lt 2
                    echo "Usage: whi delete TARGET [TARGET...]" >&2
                    return 2
                end
                __whi_apply delete $argv[2..-1]
                return $status
        end
    end

    command whi $argv
end

if not set -q WHI_SHELL_INITIALIZED
    set -gx WHI_SHELL_INITIALIZED 1
    set -gx WHI_SESSION_PID %self
    command whi __init "$WHI_SESSION_PID" 2>/dev/null
end

# IMPORTANT: Add this to the END of your fish config (~/.config/fish/config.fish):
#
#   whi init fish | source
#
# This must be at the END so whi captures your final PATH after all modifications.
#
# Also remove any old "# whi: Load saved PATH" sections from your config -
# saved PATH loading is now included at the top of this integration script.
#
# Or run in the current shell:
#
#   whi init fish | source
