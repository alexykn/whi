# whi shell integration for fish (v0.5.0)

# Absolute path to the whi binary is injected by `whi init`
set -gx __WHI_BIN "__WHI_BIN__"

function __whi_run
    $__WHI_BIN $argv
end

function __whi_apply_transition
    set -l output (__whi_run $argv)
    set -l exit_code $status
    if test $exit_code -ne 0
        return $exit_code
    end

    for line in $output
        set -l parts (string split \t -- $line)
        switch $parts[1]
            case PATH
                if test (count $parts) -ge 2
                    set -gx PATH (string split : -- $parts[2])
                end
            case SET
                if test (count $parts) -ge 3
                    set -gx $parts[2] $parts[3]
                end
            case UNSET
                if test (count $parts) -ge 2
                    set -e $parts[2]
                end
        end
    end

    return 0
end

# Load saved PATH first (if it exists) using whi
# This restores your PATH from the previous session
if test -f ~/.whi/saved_path_fish
    set -l new_path (__whi_run __load_saved_path fish)
    if test -n "$new_path"
        set -gx PATH (string split : $new_path)
    end
end

function __whi_apply
    set -l subcmd $argv[1]
    set -l rest $argv[2..-1]
    set -l new_path (__whi_run __$subcmd $rest)
    set -l exit_code $status
    if test $exit_code -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $exit_code
    end
end

function __whi_venv_source
    set -l dir $argv[1]
    test -z "$dir"; and set dir "."

    __whi_apply_transition __venv_source "$dir"
end

function __whi_venv_exit_fn
    __whi_apply_transition __venv_exit
end

function __whi_venv_unlock
    __whi_apply_transition __venv_unlock
end

function __whi_prompt
    if test -n "$WHI_VENV_NAME"
        if test "$WHI_VENV_LOCKED" = "1"
            echo "[$WHI_VENV_NAME:locked] "
        else
            echo "[$WHI_VENV_NAME] "
        end
    end
end

function __whi_cd_hook --on-variable PWD
    # Get auto-activation config
    set -l config (__whi_run __should_auto_activate)
    set -l auto_file 0
    set -l auto_lock 0

    if string match -qr 'file=1' -- $config
        set auto_file 1
    end
    if string match -qr 'lock=1' -- $config
        set auto_lock 1
    end

    # Check if we should auto-activate or auto-deactivate
    set -l has_lock 0
    set -l has_file 0
    test -f "$PWD/whi.lock"; and set has_lock 1
    test -f "$PWD/whi.file"; and set has_file 1

    # If already in a venv, check if we left that directory
    if test -n "$WHI_VENV_DIR"
        if test "$PWD" != "$WHI_VENV_DIR"
            # Left venv directory, deactivate
            __whi_venv_exit_fn 2>/dev/null
        end
    end

    # Auto-activate if configured and not already in venv
    if test -z "$WHI_VENV_NAME"
        if test $auto_lock -eq 1 -a $has_lock -eq 1
            __whi_venv_source "$PWD"
        else if test $auto_file -eq 1 -a $has_file -eq 1
            __whi_venv_source "$PWD"
        end
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
    __whi_run --all $argv
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
            case source
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi source"
                    echo "  Activate venv from whi.file or whi.lock in current directory"
                    return 0
                end
                if test (count $argv) -ne 1
                    echo "Usage: whi source" >&2
                    return 2
                end
                __whi_venv_source "$PWD"
                return $status
            case exit
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi exit"
                    echo "  Exit active venv and restore previous PATH"
                    return 0
                end
                if test (count $argv) -ne 1
                    echo "Usage: whi exit" >&2
                    return 2
                end
                __whi_venv_exit_fn
                return $status
            case unlock
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi unlock"
                    echo "  Convert whi.lock back to whi.file without leaving the environment"
                    return 0
                end
                if test (count $argv) -ne 1
                    echo "Usage: whi unlock" >&2
                    return 2
                end
                __whi_venv_unlock
                return $status
        end
    end

    __whi_run $argv
end

if not set -q __whi_prompt_installed
    set -g __whi_prompt_installed 1

    # Detect if using Starship or other prompt frameworks
    if type -q starship
        # Starship detected - use fish_right_prompt to avoid conflicts
        if not functions -q __whi_original_fish_right_prompt
            if functions -q fish_right_prompt
                functions -c fish_right_prompt __whi_original_fish_right_prompt
            else
                function __whi_original_fish_right_prompt
                    # Empty default
                end
            end
        end

        function fish_right_prompt
            set -l whi_part (__whi_prompt)
            set -l orig (__whi_original_fish_right_prompt)
            if test -n "$whi_part"
                echo -n "$whi_part"
            end
            if test -n "$orig"
                echo -n "$orig"
            end
        end
    else
        # No Starship - use traditional fish_prompt wrapping
        if not functions -q __whi_original_fish_prompt
            if functions -q fish_prompt
                functions -c fish_prompt __whi_original_fish_prompt
            else if functions -q fish_default_prompt
                functions -c fish_default_prompt __whi_original_fish_prompt
            else
                function __whi_original_fish_prompt
                    set -l pwd (prompt_pwd)
                    printf '%s> ' $pwd
                end
            end
        end

        function fish_prompt
            set -l last_status $status
            echo -n (__whi_prompt)
            __whi_original_fish_prompt
            return $last_status
        end
    end
end

if not set -q WHI_SHELL_INITIALIZED
    set -gx WHI_SHELL_INITIALIZED 1
    set -gx WHI_SESSION_PID %self
    __whi_run __init "$WHI_SESSION_PID"
end

# IMPORTANT: Add this to the END of your fish config (~/.config/fish/config.fish):
#   whi init fish | source
# This must be at the END so whi captures your final PATH after all modifications.
# Also remove any old "# whi: Load saved PATH" sections from your config -
# saved PATH loading is now included at the top of this integration script.
# Or run in the current shell:
#   whi init fish | source

# Prompt integration: whi automatically prepends "[name]" or "[name:locked]".
# Customize by editing __whi_prompt or overriding fish_prompt after sourcing if
# you prefer a different placement.

