use std::env;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

pub struct EnvLock {
    path: PathBuf,
}

impl Drop for EnvLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

/// Acquire a process-wide lock used by tests that mutate environment variables.
///
/// # Panics
///
/// Panics if the lock file cannot be created after repeated retries.
#[must_use]
pub fn env_lock() -> EnvLock {
    let path = env::temp_dir().join("whi-test-env.lock");

    loop {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return EnvLock { path },
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                thread::sleep(Duration::from_millis(10));
            }
            Err(err) => panic!("failed to acquire env lock: {err}"),
        }
    }
}

pub struct EnvVarGuard {
    key: String,
    old: Option<OsString>,
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        unsafe {
            match &self.old {
                Some(value) => env::set_var(&self.key, value),
                None => env::remove_var(&self.key),
            }
        }
    }
}

pub fn set_env_var(key: &str, value: impl AsRef<OsStr>) -> EnvVarGuard {
    let old = env::var_os(key);
    unsafe {
        env::set_var(key, value.as_ref());
    }
    EnvVarGuard {
        key: key.to_string(),
        old,
    }
}
