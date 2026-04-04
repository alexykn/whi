mod support;

use std::path::PathBuf;

use support::{env_lock, set_env_var};
use tempfile::TempDir;
use whi::config::{protected_paths, runtime, shell_paths};
use whi::path::file::parse_path_file;
use whi::shell::detect::{Shell, get_config_file_path, get_saved_path_file, get_sourcing_line};

#[test]
fn runtime_default_config_and_load() {
    let _lock = env_lock();
    let home = TempDir::new().unwrap();
    let _home = set_env_var("HOME", home.path());

    runtime::ensure_config_exists().unwrap();
    let config = runtime::load_config().unwrap();

    assert!(!config.search.executable_search_fuzzy);
    assert!(home.path().join(".whi/config.toml").exists());
}

#[test]
fn protected_paths_roundtrip() {
    let _lock = env_lock();
    let home = TempDir::new().unwrap();
    let _home = set_env_var("HOME", home.path());

    let paths = vec![PathBuf::from("/usr/bin"), PathBuf::from("/bin")];
    protected_paths::save_protected_paths(&paths).unwrap();
    let loaded = protected_paths::load_protected_paths().unwrap();

    assert_eq!(loaded, paths);
}

#[test]
fn shell_paths_backup_and_load_saved_path() {
    let _lock = env_lock();
    let home = TempDir::new().unwrap();
    let _home = set_env_var("HOME", home.path());

    shell_paths::save_path(&Shell::Bash, "/usr/bin:/bin").unwrap();
    shell_paths::save_path(&Shell::Bash, "/bin:/usr/bin").unwrap();

    let loaded = shell_paths::load_saved_path_for_shell(&Shell::Bash).unwrap();
    assert_eq!(loaded, "/bin:/usr/bin");

    let whi_dir = home.path().join(".whi");
    let backups: Vec<_> = std::fs::read_dir(&whi_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".bak"))
        .collect();
    assert_eq!(backups.len(), 1);
}

#[test]
fn shell_detect_parsing_and_paths() {
    let _lock = env_lock();
    let home = TempDir::new().unwrap();
    let _home = set_env_var("HOME", home.path());

    assert_eq!("bash".parse::<Shell>().unwrap(), Shell::Bash);
    assert_eq!("BASH".parse::<Shell>().unwrap(), Shell::Bash);
    assert_eq!("zsh".parse::<Shell>().unwrap(), Shell::Zsh);
    assert_eq!("fish".parse::<Shell>().unwrap(), Shell::Fish);
    assert!("invalid".parse::<Shell>().is_err());

    assert_eq!(Shell::Bash.as_str(), "bash");
    assert_eq!(Shell::Zsh.as_str(), "zsh");
    assert_eq!(Shell::Fish.as_str(), "fish");

    let config_file = get_config_file_path(&Shell::Fish).unwrap();
    assert_eq!(config_file, home.path().join(".config/fish/config.fish"));

    let saved_path_file = get_saved_path_file(&Shell::Fish).unwrap();
    assert_eq!(saved_path_file, home.path().join(".whi/saved_path_fish"));

    let sourcing_line = get_sourcing_line(&Shell::Fish).unwrap();
    let parsed = parse_path_file("!path.replace\n/usr/bin\n").unwrap();
    assert_eq!(
        parsed.path.replace.as_ref().unwrap(),
        &vec!["/usr/bin".to_string()]
    );
    assert!(sourcing_line.contains("whi: Load saved PATH"));
}
