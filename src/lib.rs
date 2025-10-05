#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

pub mod app;
pub mod atomic_file;
pub mod cli;
pub mod config;
pub mod config_manager;
pub mod executor;
pub mod file_utils;
pub mod history;
pub mod output;
pub mod path;
pub mod path_diff;
pub mod path_file;
pub mod path_guard;
pub mod path_resolver;
pub mod protected_config;
pub mod session_tracker;
pub mod shell_detect;
pub mod shell_integration;
pub mod system;
pub mod venv_manager;

#[cfg(test)]
pub(crate) mod test_utils {
    use std::sync::{Mutex, OnceLock};

    /// Global lock to serialize tests that mutate process-wide environment variables.
    #[must_use]
    pub(crate) fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }
}
