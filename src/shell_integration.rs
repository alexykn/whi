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
    if [ "$#" -ne 2 ]; then
        echo "Usage: whip NAME INDEX" >&2
        return 2
    fi

    local new_path
    new_path=$(whi --prefer "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whia() {
    whi -ia "$@"
}

whii() {
    whi -i "$@"
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
        echo "Usage: whid INDEX..." >&2
        return 2
    fi

    local new_path
    new_path=$(whi --delete "$@")
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
    if [ "$#" -ne 2 ]; then
        echo "Usage: whip NAME INDEX" >&2
        return 2
    fi

    local new_path
    new_path=$(whi --prefer "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whia() {
    whi -ia "$@"
}

whii() {
    whi -i "$@"
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
        echo "Usage: whid INDEX..." >&2
        return 2
    fi

    local new_path
    new_path=$(whi --delete "$@")
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
    if test (count $argv) -ne 2
        echo "Usage: whip NAME INDEX" >&2
        return 2
    end

    set -l new_path (whi --prefer $argv[1] $argv[2])
    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end

function whia
    whi -ia $argv
end

function whii
    whi -i $argv
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
        echo "Usage: whid INDEX..." >&2
        return 2
    end

    set -l new_path (whi --delete $argv)
    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end
"#;
