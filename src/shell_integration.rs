pub fn generate_init_script(shell: &str) -> Result<String, String> {
    match shell {
        "bash" => Ok(BASH_INIT.to_string()),
        "zsh" => Ok(ZSH_INIT.to_string()),
        "fish" => Ok(FISH_INIT.to_string()),
        _ => Err(format!("Unsupported shell: {shell}")),
    }
}

const BASH_INIT: &str = r#"# whicha shell integration for bash

whichm() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whichm FROM TO" >&2
        return 2
    fi

    local new_path
    new_path=$(whicha --move "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whichs() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whichs IDX1 IDX2" >&2
        return 2
    fi

    local new_path
    new_path=$(whicha --swap "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}
"#;

const ZSH_INIT: &str = r#"# whicha shell integration for zsh

whichm() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whichm FROM TO" >&2
        return 2
    fi

    local new_path
    new_path=$(whicha --move "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}

whichs() {
    if [ "$#" -ne 2 ]; then
        echo "Usage: whichs IDX1 IDX2" >&2
        return 2
    fi

    local new_path
    new_path=$(whicha --swap "$1" "$2")
    if [ $? -eq 0 ]; then
        export PATH="$new_path"
    else
        return $?
    fi
}
"#;

const FISH_INIT: &str = r#"# whicha shell integration for fish

function whichm
    if test (count $argv) -ne 2
        echo "Usage: whichm FROM TO" >&2
        return 2
    end

    set -l new_path (whicha --move $argv[1] $argv[2])
    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end

function whichs
    if test (count $argv) -ne 2
        echo "Usage: whichs IDX1 IDX2" >&2
        return 2
    end

    set -l new_path (whicha --swap $argv[1] $argv[2])
    if test $status -eq 0
        set -gx PATH (string split : $new_path)
    else
        return $status
    end
end
"#;
