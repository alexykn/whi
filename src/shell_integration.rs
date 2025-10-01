pub fn generate_init_script(shell: &str) -> Result<String, String> {
    match shell {
        "bash" => Ok(BASH_INIT.to_string()),
        "zsh" => Ok(ZSH_INIT.to_string()),
        "fish" => Ok(FISH_INIT.to_string()),
        _ => Err(format!("Unsupported shell: {shell}")),
    }
}

const BASH_INIT: &str = r#"# whi shell integration for bash (v0.3.0)

__whi_apply_path() {
    local subcmd="$1"
    shift
    local new_path
    new_path=$(command whi "__${subcmd}" "$@")
    local status=$?
    if [ $status -eq 0 ]; then
        export PATH="$new_path"
    else
        return $status
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

    __whi_apply_path swap "$1" "$2"
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
        echo "Examples:" >&2
        echo "  whip ~/.cargo/bin           # Add path to PATH (if not present)" >&2
        echo "  whip cargo 3                # Use cargo from PATH index 3" >&2
        echo "  whip cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo" >&2
        echo "  whip bat github release     # Use bat from path matching pattern" >&2
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
        echo "  Fuzzy patterns delete ALL matching entries" >&2
        echo "Examples:" >&2
        echo "  whid 3                      # Delete PATH entry at index 3" >&2
        echo "  whid 2 5 7                  # Delete multiple indices" >&2
        echo "  whid ~/.local/bin           # Delete ~/.local/bin from PATH" >&2
        echo "  whid temp bin               # Delete ALL entries matching pattern" >&2
        return 2
    fi

    __whi_apply_path delete "$@"
}

whia() {
    command whi --all "$@"
}

whi() {
    if [ "$#" -gt 0 ]; then
        case "$1" in
            prefer)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi prefer [NAME] TARGET [PATTERN...]"
                        echo "  Add path to PATH or prefer executable at target"
                        echo "  TARGET can be index, path, or fuzzy pattern"
                        echo "Examples:"
                        echo "  whi prefer ~/.cargo/bin           # Add path to PATH (if not present)"
                        echo "  whi prefer cargo 3                # Use cargo from PATH index 3"
                        echo "  whi prefer cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo"
                        echo "  whi prefer bat github release     # Use bat from path matching pattern"
                        return 0
                        ;;
                esac
                if [ "$#" -lt 1 ]; then
                    echo "Usage: whi prefer [NAME] TARGET [PATTERN...]" >&2
                    echo "  Add path to PATH or prefer executable at target" >&2
                    echo "  TARGET can be index, path, or fuzzy pattern" >&2
                    echo "Examples:" >&2
                    echo "  whi prefer ~/.cargo/bin           # Add path to PATH (if not present)" >&2
                    echo "  whi prefer cargo 3                # Use cargo from PATH index 3" >&2
                    echo "  whi prefer cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo" >&2
                    echo "  whi prefer bat github release     # Use bat from path matching pattern" >&2
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
                    echo "  Move PATH entry from index FROM to index TO" >&2
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
                    echo "  Swap PATH entries at indices IDX1 and IDX2" >&2
                    return 2
                fi
                __whi_apply_path swap "$@"
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
                    echo "  Remove duplicate entries from PATH" >&2
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
                        echo "  Fuzzy patterns delete ALL matching entries"
                        echo "Examples:"
                        echo "  whi delete 3                      # Delete PATH entry at index 3"
                        echo "  whi delete 2 5 7                  # Delete multiple indices"
                        echo "  whi delete ~/.local/bin           # Delete ~/.local/bin from PATH"
                        echo "  whi delete temp bin               # Delete ALL entries matching pattern"
                        return 0
                        ;;
                esac
                if [ "$#" -lt 1 ]; then
                    echo "Usage: whi delete TARGET [TARGET...]" >&2
                    echo "  TARGET can be index, path, or fuzzy pattern" >&2
                    echo "  Fuzzy patterns delete ALL matching entries" >&2
                    echo "Examples:" >&2
                    echo "  whi delete 3                      # Delete PATH entry at index 3" >&2
                    echo "  whi delete 2 5 7                  # Delete multiple indices" >&2
                    echo "  whi delete ~/.local/bin           # Delete ~/.local/bin from PATH" >&2
                    echo "  whi delete temp bin               # Delete ALL entries matching pattern" >&2
                    return 2
                fi
                __whi_apply_path delete "$@"
                return $?
                ;;
        esac
    fi

    command whi "$@"
}

export WHI_SHELL_INITIALIZED=1

# Add this to your shell config (~/.bashrc, ~/.zshrc, etc.):
#
#   eval "$(whi init bash)"
#
# Or run in the current shell:
#
#   eval "$(whi init bash)"
"#;
const ZSH_INIT: &str = r#"# whi shell integration for zsh (v0.3.0)

__whi_apply_path() {
    local subcmd="$1"
    shift
    local new_path
    new_path=$(command whi "__${subcmd}" "$@")
    local status=$?
    if [ $status -eq 0 ]; then
        export PATH="$new_path"
    else
        return $status
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

    __whi_apply_path swap "$1" "$2"
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
        echo "Examples:" >&2
        echo "  whip ~/.cargo/bin           # Add path to PATH (if not present)" >&2
        echo "  whip cargo 3                # Use cargo from PATH index 3" >&2
        echo "  whip cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo" >&2
        echo "  whip bat github release     # Use bat from path matching pattern" >&2
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
        echo "  Fuzzy patterns delete ALL matching entries" >&2
        echo "Examples:" >&2
        echo "  whid 3                      # Delete PATH entry at index 3" >&2
        echo "  whid 2 5 7                  # Delete multiple indices" >&2
        echo "  whid ~/.local/bin           # Delete ~/.local/bin from PATH" >&2
        echo "  whid temp bin               # Delete ALL entries matching pattern" >&2
        return 2
    fi

    __whi_apply_path delete "$@"
}

whia() {
    command whi --all "$@"
}

whi() {
    if [ "$#" -gt 0 ]; then
        case "$1" in
            prefer)
                shift
                case "$1" in
                    help|--help|-h)
                        echo "Usage: whi prefer [NAME] TARGET [PATTERN...]"
                        echo "  Add path to PATH or prefer executable at target"
                        echo "  TARGET can be index, path, or fuzzy pattern"
                        echo "Examples:"
                        echo "  whi prefer ~/.cargo/bin           # Add path to PATH (if not present)"
                        echo "  whi prefer cargo 3                # Use cargo from PATH index 3"
                        echo "  whi prefer cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo"
                        echo "  whi prefer bat github release     # Use bat from path matching pattern"
                        return 0
                        ;;
                esac
                if [ "$#" -lt 1 ]; then
                    echo "Usage: whi prefer [NAME] TARGET [PATTERN...]" >&2
                    echo "  Add path to PATH or prefer executable at target" >&2
                    echo "  TARGET can be index, path, or fuzzy pattern" >&2
                    echo "Examples:" >&2
                    echo "  whi prefer ~/.cargo/bin           # Add path to PATH (if not present)" >&2
                    echo "  whi prefer cargo 3                # Use cargo from PATH index 3" >&2
                    echo "  whi prefer cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo" >&2
                    echo "  whi prefer bat github release     # Use bat from path matching pattern" >&2
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
                    echo "  Move PATH entry from index FROM to index TO" >&2
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
                    echo "  Swap PATH entries at indices IDX1 and IDX2" >&2
                    return 2
                fi
                __whi_apply_path swap "$@"
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
                    echo "  Remove duplicate entries from PATH" >&2
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
                        echo "  Fuzzy patterns delete ALL matching entries"
                        echo "Examples:"
                        echo "  whi delete 3                      # Delete PATH entry at index 3"
                        echo "  whi delete 2 5 7                  # Delete multiple indices"
                        echo "  whi delete ~/.local/bin           # Delete ~/.local/bin from PATH"
                        echo "  whi delete temp bin               # Delete ALL entries matching pattern"
                        return 0
                        ;;
                esac
                if [ "$#" -lt 1 ]; then
                    echo "Usage: whi delete TARGET [TARGET...]" >&2
                    echo "  TARGET can be index, path, or fuzzy pattern" >&2
                    echo "  Fuzzy patterns delete ALL matching entries" >&2
                    echo "Examples:" >&2
                    echo "  whi delete 3                      # Delete PATH entry at index 3" >&2
                    echo "  whi delete 2 5 7                  # Delete multiple indices" >&2
                    echo "  whi delete ~/.local/bin           # Delete ~/.local/bin from PATH" >&2
                    echo "  whi delete temp bin               # Delete ALL entries matching pattern" >&2
                    return 2
                fi
                __whi_apply_path delete "$@"
                return $?
                ;;
        esac
    fi

    command whi "$@"
}

export WHI_SHELL_INITIALIZED=1

# Add this to your shell config (~/.zshrc, ~/.bashrc, etc.):
#
#   eval "$(whi init zsh)"
#
# Or run in the current shell:
#
#   eval "$(whi init zsh)"
"#;
const FISH_INIT: &str = r#"# whi shell integration for fish (v0.3.0)

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
        echo "  Move PATH entry from index FROM to index TO" >&2
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
        echo "  Swap PATH entries at indices IDX1 and IDX2" >&2
        return 2
    end

    __whi_apply swap $argv
end

function whip
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whip [NAME] TARGET [PATTERN...]"
        echo "  Add path to PATH or prefer executable at target"
        echo "  TARGET can be index, path, or fuzzy pattern"
        echo "Examples:"
        echo "  whip ~/.cargo/bin           # Add path to PATH (if not present)"
        echo "  whip cargo 3                # Use cargo from PATH index 3"
        echo "  whip cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo"
        echo "  whip bat github release     # Use bat from path matching pattern"
        return 0
    end
    if test (count $argv) -lt 1
        echo "Usage: whip [NAME] TARGET [PATTERN...]" >&2
        echo "  Add path to PATH or prefer executable at target" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        echo "Examples:" >&2
        echo "  whip ~/.cargo/bin           # Add path to PATH (if not present)" >&2
        echo "  whip cargo 3                # Use cargo from PATH index 3" >&2
        echo "  whip cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo" >&2
        echo "  whip bat github release     # Use bat from path matching pattern" >&2
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
        echo "  Remove duplicate entries from PATH" >&2
        return 2
    end
    __whi_apply clean
end

function whid
    if test (count $argv) -ge 1; and contains -- $argv[1] help --help -h
        echo "Usage: whid TARGET [TARGET...]"
        echo "  TARGET can be index, path, or fuzzy pattern"
        echo "  Fuzzy patterns delete ALL matching entries"
        echo "Examples:"
        echo "  whid 3                      # Delete PATH entry at index 3"
        echo "  whid 2 5 7                  # Delete multiple indices"
        echo "  whid ~/.local/bin           # Delete ~/.local/bin from PATH"
        echo "  whid temp bin               # Delete ALL entries matching pattern"
        return 0
    end
    if test (count $argv) -lt 1
        echo "Usage: whid TARGET [TARGET...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        echo "  Fuzzy patterns delete ALL matching entries" >&2
        echo "Examples:" >&2
        echo "  whid 3                      # Delete PATH entry at index 3" >&2
        echo "  whid 2 5 7                  # Delete multiple indices" >&2
        echo "  whid ~/.local/bin           # Delete ~/.local/bin from PATH" >&2
        echo "  whid temp bin               # Delete ALL entries matching pattern" >&2
        return 2
    end

    __whi_apply delete $argv
end

function whia
    command whi --all $argv
end

function whi
    if test (count $argv) -gt 0
        switch $argv[1]
            case prefer
                # Check for help request
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi prefer [NAME] TARGET [PATTERN...]"
                    echo "  Add path to PATH or prefer executable at target"
                    echo "  TARGET can be index, path, or fuzzy pattern"
                    echo "Examples:"
                    echo "  whi prefer ~/.cargo/bin           # Add path to PATH (if not present)"
                    echo "  whi prefer cargo 3                # Use cargo from PATH index 3"
                    echo "  whi prefer cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo"
                    echo "  whi prefer bat github release     # Use bat from path matching pattern"
                    return 0
                end
                if test (count $argv) -lt 2
                    echo "Usage: whi prefer [NAME] TARGET [PATTERN...]" >&2
                    echo "  Add path to PATH or prefer executable at target" >&2
                    echo "  TARGET can be index, path, or fuzzy pattern" >&2
                    echo "Examples:" >&2
                    echo "  whi prefer ~/.cargo/bin           # Add path to PATH (if not present)" >&2
                    echo "  whi prefer cargo 3                # Use cargo from PATH index 3" >&2
                    echo "  whi prefer cargo ~/.cargo/bin     # Add/prefer ~/.cargo/bin for cargo" >&2
                    echo "  whi prefer bat github release     # Use bat from path matching pattern" >&2
                    return 2
                end
                __whi_apply prefer $argv[2..-1]
                return $status
            case move
                # Check for help request
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi move FROM TO"
                    echo "  Move PATH entry from index FROM to index TO"
                    return 0
                end
                if test (count $argv) -ne 3
                    echo "Usage: whi move FROM TO" >&2
                    echo "  Move PATH entry from index FROM to index TO" >&2
                    return 2
                end
                __whi_apply move $argv[2..-1]
                return $status
            case switch
                # Check for help request
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi switch IDX1 IDX2"
                    echo "  Swap PATH entries at indices IDX1 and IDX2"
                    return 0
                end
                if test (count $argv) -ne 3
                    echo "Usage: whi switch IDX1 IDX2" >&2
                    echo "  Swap PATH entries at indices IDX1 and IDX2" >&2
                    return 2
                end
                __whi_apply swap $argv[2..-1]
                return $status
            case clean
                # Check for help request
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi clean"
                    echo "  Remove duplicate entries from PATH"
                    return 0
                end
                if test (count $argv) -ne 1
                    echo "Usage: whi clean" >&2
                    echo "  Remove duplicate entries from PATH" >&2
                    return 2
                end
                __whi_apply clean $argv[2..-1]
                return $status
            case delete
                # Check for help request
                if test (count $argv) -ge 2; and contains -- $argv[2] help --help -h
                    echo "Usage: whi delete TARGET [TARGET...]"
                    echo "  TARGET can be index, path, or fuzzy pattern"
                    echo "  Fuzzy patterns delete ALL matching entries"
                    echo "Examples:"
                    echo "  whi delete 3                      # Delete PATH entry at index 3"
                    echo "  whi delete 2 5 7                  # Delete multiple indices"
                    echo "  whi delete ~/.local/bin           # Delete ~/.local/bin from PATH"
                    echo "  whi delete temp bin               # Delete ALL entries matching pattern"
                    return 0
                end
                if test (count $argv) -lt 2
                    echo "Usage: whi delete TARGET [TARGET...]" >&2
                    echo "  TARGET can be index, path, or fuzzy pattern" >&2
                    echo "  Fuzzy patterns delete ALL matching entries" >&2
                    echo "Examples:" >&2
                    echo "  whi delete 3                      # Delete PATH entry at index 3" >&2
                    echo "  whi delete 2 5 7                  # Delete multiple indices" >&2
                    echo "  whi delete ~/.local/bin           # Delete ~/.local/bin from PATH" >&2
                    echo "  whi delete temp bin               # Delete ALL entries matching pattern" >&2
                    return 2
                end
                __whi_apply delete $argv[2..-1]
                return $status
        end
    end

    command whi $argv
end

set -gx WHI_SHELL_INITIALIZED 1

# Add this to your fish config (~/.config/fish/config.fish):
#
#   whi init fish | source
#
# Or run in the current shell:
#
#   whi init fish | source
"#;
