pub fn generate_init_script(shell: &str) -> Result<String, String> {
    let whi_path = std::env::current_exe()
        .map_err(|e| format!("Failed to get whi executable path: {e}"))?
        .to_string_lossy()
        .to_string();

    let escaped = escape_for_double_quotes(&whi_path);

    let script = match shell {
        "bash" | "zsh" => POSIX_INIT.replace("__WHI_BIN__", &escaped),
        "fish" => FISH_INIT.replace("__WHI_BIN__", &escaped),
        _ => return Err(format!("Unsupported shell: {shell}")),
    };

    Ok(script)
}

const POSIX_INIT: &str = include_str!("posix_integration.sh");
const FISH_INIT: &str = include_str!("fish_integration.fish");

fn escape_for_double_quotes(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '$' => escaped.push_str("\\$"),
            '`' => escaped.push_str("\\`"),
            _ => escaped.push(ch),
        }
    }
    escaped
}
