pub fn generate_init_script(shell: &str) -> Result<String, String> {
    match shell {
        "bash" | "zsh" => Ok(POSIX_INIT.to_string()),
        "fish" => Ok(FISH_INIT.to_string()),
        _ => Err(format!("Unsupported shell: {shell}")),
    }
}

const POSIX_INIT: &str = include_str!("posix_integration.sh");
const FISH_INIT: &str = include_str!("fish_integration.fish");
