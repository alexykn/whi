pub fn generate_init_script(shell: &str) -> Result<String, String> {
    match shell {
        "bash" => Ok(BASH_INIT.to_string()),
        "zsh" => Ok(ZSH_INIT.to_string()),
        "fish" => Ok(FISH_INIT.to_string()),
        _ => Err(format!("Unsupported shell: {shell}")),
    }
}

const BASH_INIT: &str = r#"# whi shell integration for bash

whim() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whim FROM TO" >&2
        return 2
    fi

    local new_path
    new_path=$(whi --move "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whis() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whis IDX1 IDX2" >&2
        return 2
    fi

    local new_path
    new_path=$(whi --swap "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whip() {
    if [ "$#" -lt 1 ]; then
        echo "Usage: whip [NAME] TARGET [PATTERN...]" >&2
        echo "  Add path to PATH or prefer executable at target" >&2
        echo "Examples:" >&2
        echo "  whip ~/.cargo/bin         # Add path to PATH (if not present)" >&2
        echo "  whip cargo 3              # Use cargo from PATH index 3" >&2
        echo "  whip cargo ~/.cargo/bin   # Add/prefer ~/.cargo/bin for cargo" >&2
        echo "  whip bat github release   # Use bat from path matching pattern" >&2
        return 2
    fi

    local new_path

    # Check if single argument that looks like a path
    if [ "$#" -eq 1 ] && [[ "$1" =~ [/~.] ]]; then
        # Path-only mode
        new_path=$(whi --prefer "$1")
    else
        # Traditional mode with name and target
        local name="$1"
        shift

        # If multiple args, join them as fuzzy pattern
        local target="$*"

        new_path=$(whi --prefer "$name" "$target")
    fi

    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whia() {
    whi -ia "$@"
}

whic() {
    local new_path
    new_path=$(whi --clean)
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whid() {
    if [ "$#" -lt 1 ]; then
        echo "Usage: whid TARGET [ARGS...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        echo "  Cannot mix indices and paths in one command" >&2
        echo "Examples:" >&2
        echo "  whid 3                  # Delete PATH entry at index 3" >&2
        echo "  whid ~/.local/bin       # Delete ~/.local/bin from PATH" >&2
        echo "  whid temp bin           # Delete ALL entries matching pattern" >&2
        echo "  whid 2 5 7              # Delete multiple indices" >&2
        return 2
    fi

    # Check if all arguments are numbers (indices)
    local all_numeric=1
    for arg in "$@"; do
        if ! [[ "$arg" =~ ^[0-9]+$ ]]; then
            all_numeric=0
            break
        fi
    done

    local new_path
    if [ "$all_numeric" -eq 1 ]; then
        # All numeric - pass as separate indices
        new_path=$(whi --delete "$@")
    else
        # Has non-numeric - join as single fuzzy pattern
        new_path=$(whi --delete "$*")
    fi

    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}
"#;

const ZSH_INIT: &str = r#"# whi shell integration for zsh

whim() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whim FROM TO" >&2
        return 2
    fi

    local new_path
    new_path=$(whi --move "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whis() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whis IDX1 IDX2" >&2
        return 2
    fi

    local new_path
    new_path=$(whi --swap "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whip() {
    if [ "$#" -lt 1 ]; then
        echo "Usage: whip [NAME] TARGET [PATTERN...]" >&2
        echo "  Add path to PATH or prefer executable at target" >&2
        echo "Examples:" >&2
        echo "  whip ~/.cargo/bin         # Add path to PATH (if not present)" >&2
        echo "  whip cargo 3              # Use cargo from PATH index 3" >&2
        echo "  whip cargo ~/.cargo/bin   # Add/prefer ~/.cargo/bin for cargo" >&2
        echo "  whip bat github release   # Use bat from path matching pattern" >&2
        return 2
    fi

    local new_path

    # Check if single argument that looks like a path
    if [ "$#" -eq 1 ] && [[ "$1" =~ [/~.] ]]; then
        # Path-only mode
        new_path=$(whi --prefer "$1")
    else
        # Traditional mode with name and target
        local name="$1"
        shift

        # If multiple args, join them as fuzzy pattern
        local target="$*"

        new_path=$(whi --prefer "$name" "$target")
    fi

    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whia() {
    whi -ia "$@"
}

whic() {
    local new_path
    new_path=$(whi --clean)
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whid() {
    if [ "$#" -lt 1 ]; then
        echo "Usage: whid TARGET [ARGS...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        echo "  Cannot mix indices and paths in one command" >&2
        echo "Examples:" >&2
        echo "  whid 3                  # Delete PATH entry at index 3" >&2
        echo "  whid ~/.local/bin       # Delete ~/.local/bin from PATH" >&2
        echo "  whid temp bin           # Delete ALL entries matching pattern" >&2
        echo "  whid 2 5 7              # Delete multiple indices" >&2
        return 2
    fi

    # Check if all arguments are numbers (indices)
    local all_numeric=1
    for arg in "$@"; do
        if ! [[ "$arg" =~ ^[0-9]+$ ]]; then
            all_numeric=0
            break
        fi
    done

    local new_path
    if [ "$all_numeric" -eq 1 ]; then
        # All numeric - pass as separate indices
        new_path=$(whi --delete "$@")
    else
        # Has non-numeric - join as single fuzzy pattern
        new_path=$(whi --delete "$*")
    fi

    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}
"#;

const FISH_INIT: &str = r#"# whi shell integration for fish

function whim
    if test (count $argv) -ne 2
        echo "Usage: whim FROM TO" >&2
        return 2
    end

    set -l new_path (whi --move $argv[1] $argv[2])
    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end

function whis
    if test (count $argv) -ne 2
        echo "Usage: whis IDX1 IDX2" >&2
        return 2
    end

    set -l new_path (whi --swap $argv[1] $argv[2])
    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end

function whip
    if test (count $argv) -lt 1
        echo "Usage: whip [NAME] TARGET [PATTERN...]" >&2
        echo "  Add path to PATH or prefer executable at target" >&2
        echo "Examples:" >&2
        echo "  whip ~/.cargo/bin         # Add path to PATH (if not present)" >&2
        echo "  whip cargo 3              # Use cargo from PATH index 3" >&2
        echo "  whip cargo ~/.cargo/bin   # Add/prefer ~/.cargo/bin for cargo" >&2
        echo "  whip bat github release   # Use bat from path matching pattern" >&2
        return 2
    end

    set -l new_path

    # Check if single argument that looks like a path
    if test (count $argv) -eq 1 -a (string match -qr '[/~.]' -- $argv[1])
        # Path-only mode
        set new_path (whi --prefer $argv[1])
    else
        # Traditional mode with name and target
        set -l name $argv[1]
        set -l target $argv[2..]

        # Join multiple args as fuzzy pattern
        set -l target_str (string join ' ' $target)

        set new_path (whi --prefer $name $target_str)
    end

    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end

function whia
    whi -ia $argv
end

function whic
    set -l new_path (whi --clean)
    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end

function whid
    if test (count $argv) -lt 1
        echo "Usage: whid TARGET [ARGS...]" >&2
        echo "  TARGET can be index, path, or fuzzy pattern" >&2
        echo "  Cannot mix indices and paths in one command" >&2
        echo "Examples:" >&2
        echo "  whid 3                  # Delete PATH entry at index 3" >&2
        echo "  whid ~/.local/bin       # Delete ~/.local/bin from PATH" >&2
        echo "  whid temp bin           # Delete ALL entries matching pattern" >&2
        echo "  whid 2 5 7              # Delete multiple indices" >&2
        return 2
    end

    # Check if all arguments are numbers (indices)
    set -l all_numeric 1
    for arg in $argv
        if not string match -qr '^[0-9]+$' -- $arg
            set all_numeric 0
            break
        end
    end

    set -l new_path
    if test $all_numeric -eq 1
        # All numeric - pass as separate indices
        set new_path (whi --delete $argv)
    else
        # Has non-numeric - join as single fuzzy pattern
        set new_path (whi --delete (string join ' ' $argv))
    end

    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end
"#;
