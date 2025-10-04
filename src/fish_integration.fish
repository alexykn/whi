# whi shell integration for fish (v0.6.1)

# Absolute path to the whi binary is injected by `whi init`
set -gx __WHI_BIN "__WHI_BIN__"
set -g __WHI_CONFIG_PATH "$HOME/.whi/config.toml"
set -g __WHI_AUTO_FILE 0
set -g __WHI_AUTO_FILE_MTIME ""
set -g __WHI_STAT_STYLE ""
set -g __WHI_STAT_SKIP 0

function __whi_run
    $__WHI_BIN $argv
end

function __whi_detect_stat_style
    if test -n "$__WHI_STAT_STYLE"
        return
    end

    set -l probe $HOME
    if test -z "$probe"
        set probe .
    end

    command stat -c %Y $probe > /dev/null 2>&1
    if test $status -eq 0
        set -g __WHI_STAT_STYLE gnu
        return
    end

    command stat -f %m $probe > /dev/null 2>&1
    if test $status -eq 0
        set -g __WHI_STAT_STYLE bsd
        return
    end

    set -g __WHI_STAT_STYLE none
end

function __whi_stat_mtime
    set -l path $argv[1]

    if test -z "$path"
        return 0
    end

    if not test -e "$path"
        return 0
    end

    __whi_detect_stat_style

    switch $__WHI_STAT_STYLE
        case gnu
            set -l result (command stat -c %Y $path 2>/dev/null)
            if test $status -eq 0 -a -n "$result"
                echo $result
            end
        case bsd
            set -l result (command stat -f %m $path 2>/dev/null)
            if test $status -eq 0 -a -n "$result"
                echo $result
            end
    end

    return 0
end

function __whi_refresh_auto_config
    set -l current_mtime ""

    if test -n "$__WHI_CONFIG_PATH"
        set -l mtime (__whi_stat_mtime $__WHI_CONFIG_PATH)
        if test (count $mtime) -gt 0 -a -n "$mtime[1]"
            set current_mtime $mtime[1]
        end
    end

    if set -q __WHI_AUTO_CONFIG_LOADED
        if test "$current_mtime" = "$__WHI_AUTO_FILE_MTIME"
            return 0
        end
    end

    set -l output (__whi_run __should_auto_activate 2>/dev/null)
    set -l first (string split '\n' -- $output)[1]
    set -l auto_flag 0
    if string match -rq '^file=1' -- $first
        set auto_flag 1
    end
    set -g __WHI_AUTO_FILE $auto_flag

    if test -z "$current_mtime"
        set -g __WHI_AUTO_FILE 0
    end

    set -g __WHI_AUTO_FILE_MTIME $current_mtime
    set -g __WHI_AUTO_CONFIG_LOADED 1
    set -g __WHI_STAT_SKIP 0
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

function __whi_handle_move --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display FROM TO"
            echo "  Move PATH entry from index FROM to index TO"
            return 0
        end
    end

    if test (count $args) -ne 2
        echo "Usage: $display FROM TO" >&2
        echo "  Move PATH entry from index FROM to index TO" >&2
        return 2
    end

    __whi_apply move $args
    return $status
end

function __whi_handle_switch --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display IDX1 IDX2"
            echo "  Swap PATH entries at indices IDX1 and IDX2"
            return 0
        end
    end

    if test (count $args) -ne 2
        echo "Usage: $display IDX1 IDX2" >&2
        echo "  Swap PATH entries at indices IDX1 and IDX2" >&2
        return 2
    end

    __whi_apply switch $args
    return $status
end

function __whi_handle_clean --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display"
            echo "  Remove duplicate entries from PATH"
            return 0
        end
    end

    if test (count $args) -ne 0
        echo "Usage: $display" >&2
        echo "  Remove duplicate entries from PATH" >&2
        return 2
    end

    __whi_apply clean
    return $status
end

function __whi_handle_delete --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display TARGET [TARGET...]"
            echo "  TARGET can be index, path, or fuzzy pattern"
            echo "  Fuzzy patterns delete ALL matching entries"
            return 0
        end
    end

    if test (count $args) -lt 1
        echo "Usage: $display TARGET [TARGET...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        return 2
    end

    __whi_apply delete $args
    return $status
end

function __whi_handle_add --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display PATH..."
            echo "  Add one or more paths to PATH (prepends by default)"
            return 0
        end
    end

    if test (count $args) -lt 1
        echo "Usage: $display PATH..." >&2
        return 2
    end

    __whi_apply add $args
    return $status
end

function __whi_handle_prefer --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display [NAME] TARGET [PATTERN...]"
            echo "  Add path to PATH or prefer executable at target"
            echo "  TARGET can be index, path, or fuzzy pattern"
            echo "Examples:"
            printf '  %s ~/.cargo/bin           # Add path to PATH (if not present)\n' $display
            printf '  %s cargo 3                # Use cargo from PATH index 3\n' $display
            printf '  %s cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo\n' $display
            printf '  %s bat github release     # Use bat from path matching pattern\n' $display
            return 0
        end
    end

    if test (count $args) -lt 1
        echo "Usage: $display [NAME] TARGET [PATTERN...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        return 2
    end

    if test (count $args) -eq 1 -a (string match -qr '[/~.]' -- $args[1])
        __whi_apply prefer $args
    else
        set -l name $args[1]
        set -l rest $args[2..-1]
        __whi_apply prefer $name $rest
    end
    return $status
end

function __whi_handle_redo --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display [COUNT]"
            echo "  Redo next COUNT PATH operations (default: 1)"
            return 0
        end
    end

    if test (count $args) -eq 0
        __whi_apply redo 1
    else if test (count $args) -eq 1
        __whi_apply redo $args[1]
    else
        echo "Usage: $display [COUNT]" >&2
        echo "  Redo next COUNT PATH operations (default: 1)" >&2
        return 2
    end
    return $status
end

function __whi_handle_undo --argument-names display
    set -l args $argv[2..-1]
    if test (count $args) -ge 1
        if contains -- $args[1] help --help -h
            echo "Usage: $display [COUNT]"
            echo "  Undo last COUNT PATH operations (default: 1)"
            return 0
        end
    end

    if test (count $args) -eq 0
        __whi_apply undo 1
    else if test (count $args) -eq 1
        __whi_apply undo $args[1]
    else
        echo "Usage: $display [COUNT]" >&2
        echo "  Undo last COUNT PATH operations (default: 1)" >&2
        return 2
    end
    return $status
end

set -g __WHI_CMD_NAMES \
    move switch clean delete add prefer redo undo \
    whim whis whic whid whiad whir whiu whip
set -g __WHI_CMD_HANDLERS \
    __whi_handle_move \
    __whi_handle_switch \
    __whi_handle_clean \
    __whi_handle_delete \
    __whi_handle_add \
    __whi_handle_prefer \
    __whi_handle_redo \
    __whi_handle_undo \
    __whi_handle_move \
    __whi_handle_switch \
    __whi_handle_clean \
    __whi_handle_delete \
    __whi_handle_add \
    __whi_handle_redo \
    __whi_handle_undo \
    __whi_handle_prefer

function __whi_lookup_handler --argument-names cmd
    for idx in (seq (count $__WHI_CMD_NAMES))
        if test $__WHI_CMD_NAMES[$idx] = $cmd
            echo $__WHI_CMD_HANDLERS[$idx]
            return 0
        end
    end
    return 1
end

function __whi_dispatch --argument-names cmd
    set -l handler (__whi_lookup_handler $cmd)
    if test -z "$handler"
        return 1
    end
    set -l rest $argv[2..-1]
    $handler "whi $cmd" $rest
    return $status
end

function __whi_run_shorthand --argument-names name
    set -l handler (__whi_lookup_handler $name)
    if test -z "$handler"
        return 1
    end
    set -l rest $argv[2..-1]
    $handler $name $rest
    return $status
end

function __whi_venv_source
    set -l dir $argv[1]
    test -z "$dir"; and set dir "."

    __whi_apply_transition __venv_source "$dir"
end

function __whi_venv_exit_fn
    __whi_apply_transition __venv_exit
end

function __whi_prompt
    if test -n "$WHI_VENV_NAME"
        echo "[$WHI_VENV_NAME] "
    end
end

function __whi_cd_hook --on-variable PWD
    if not set -q __WHI_AUTO_CONFIG_LOADED
        __whi_refresh_auto_config
    end

    set -l check_config 1
    if set -q __WHI_AUTO_FILE
        if test $__WHI_AUTO_FILE -eq 0 -a -n "$__WHI_AUTO_FILE_MTIME"
            set -l skip $__WHI_STAT_SKIP
            if test $skip -lt 4
                set check_config 0
                set -g __WHI_STAT_SKIP (math "$skip + 1")
            else
                set -g __WHI_STAT_SKIP 0
            end
        else
            set -g __WHI_STAT_SKIP 0
        end
    end

    if test $check_config -eq 1 -a -n "$__WHI_CONFIG_PATH"
        set -l current_mtime (__whi_stat_mtime $__WHI_CONFIG_PATH)
        if test (count $current_mtime) -gt 0 -a -n "$current_mtime[1]"
            set -l current_val $current_mtime[1]
            if test "$current_val" != "$__WHI_AUTO_FILE_MTIME"
                __whi_refresh_auto_config
            end
        else if test -n "$__WHI_AUTO_FILE_MTIME"
            set -g __WHI_AUTO_FILE 0
            set -g __WHI_AUTO_FILE_MTIME ""
        end
        set -g __WHI_STAT_SKIP 0
    end

    # Check if we should auto-activate or auto-deactivate
    set -l has_file 0
    test -f "$PWD/whifile"; and set has_file 1

    # If already in a venv, check if we left that directory
    if test -n "$WHI_VENV_DIR"
        set -l root (string trim --right --chars='/' -- "$WHI_VENV_DIR")
        if not string match -q "$root" -- "$PWD"
            if not string match -q "$root/*" -- "$PWD"
                # Left venv directory tree, deactivate
                __whi_venv_exit_fn 2>/dev/null
            end
        end
    end

    # Auto-activate if configured and not already in venv
    if test -z "$WHI_VENV_NAME" -a $__WHI_AUTO_FILE -eq 1 -a $has_file -eq 1
        __whi_venv_source "$PWD"
    end
end

function whim
    __whi_run_shorthand whim $argv
end

function whis
    __whi_run_shorthand whis $argv
end

function whip
    __whi_run_shorthand whip $argv
end

function whic
    __whi_run_shorthand whic $argv
end

function whid
    __whi_run_shorthand whid $argv
end

function whia
    __whi_run --all $argv
end

function whiad
    __whi_run_shorthand whiad $argv
end

function whir
    __whi_run_shorthand whir $argv
end

function whiu
    __whi_run_shorthand whiu $argv
end

function whiv
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
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
    end
    __whi_run var $argv
end

function whish
    __whi_run shorthands $argv
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
    __whi_apply_transition __load $argv[1]
end

function whi
    if test (count $argv) -gt 0
        set -l cmd $argv[1]
        switch $cmd
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
                __whi_apply_transition __load $argv[2]
                return $status
            case var
                __whi_run var $argv[2..]
                return $status
            case shorthands
                __whi_run shorthands $argv[2..]
                return $status
            case source
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi source"
                    echo "  Activate venv from whifile in current directory"
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
            case '*'
                __whi_dispatch $argv
                set -l dispatch_status $status
                if test $dispatch_status -ne 1
                    return $dispatch_status
                end
        end
    end

    __whi_run $argv
end

if not set -q __whi_rprompt_installed
    set -g __whi_rprompt_installed 1

    if not functions -q __whi_original_fish_right_prompt
        if functions -q fish_right_prompt
            functions -c fish_right_prompt __whi_original_fish_right_prompt
        end
    end

    function fish_right_prompt
        set -l prefix (__whi_prompt)
        if functions -q __whi_original_fish_right_prompt
            __whi_original_fish_right_prompt
        end
        if test -n "$prefix"
            printf '%s' "$prefix"
        end
    end
end


if not set -q WHI_SHELL_INITIALIZED
    set -gx WHI_SHELL_INITIALIZED 1
    set -gx WHI_SESSION_PID %self
    __whi_run __init "$WHI_SESSION_PID"
end

# Trigger auto-activation for the current directory (if configured)
if functions -q __whi_cd_hook
    __whi_cd_hook >/dev/null
end

# IMPORTANT: Add this to the END of your fish config (~/.config/fish/config.fish):
#   whi init fish | source
# This must be at the END so whi captures your final PATH after all modifications.
# Also remove any old "# whi: Load saved PATH" sections from your config -
# saved PATH loading is now included at the top of this integration script.
# Or run in the current shell:
#   whi init fish | source

# Prompt integration: whi automatically adds "[name]" or "[name:locked]" to the right prompt.
# Customize by editing __whi_prompt or overriding fish_right_prompt after sourcing if
# you prefer a different placement.
