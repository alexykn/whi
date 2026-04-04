#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]

extern crate self as whi;

pub mod cli;
pub mod commands;
pub mod config;
pub mod io;
pub mod path;
pub mod platform;
pub mod search;
pub mod session;
pub mod shell;

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
