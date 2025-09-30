use std::io;

/// Safe wrapper for getppid() - returns parent process ID
pub fn get_parent_pid() -> io::Result<u32> {
    let ppid = unsafe { libc::getppid() };
    if ppid < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ppid as u32)
    }
}

/// Safe wrapper for getuid() - returns user ID
pub fn get_user_id() -> io::Result<u32> {
    let uid = unsafe { libc::getuid() };
    Ok(uid)
}

/// Safe wrapper for isatty() - checks if file descriptor is a terminal
pub fn is_tty(fd: i32) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_parent_pid() {
        let ppid = get_parent_pid().unwrap();
        assert!(ppid > 0, "PPID should be positive");
    }

    #[test]
    fn test_get_user_id() {
        let uid = get_user_id().unwrap();
        // UID can be 0 (root) or positive
        assert!(uid < u32::MAX, "UID should be valid");
    }

    #[test]
    fn test_is_tty() {
        // Test with stdin, stdout, stderr
        // We can't assert the result since it depends on how tests are run
        // But we can ensure it doesn't panic
        let _ = is_tty(0);
        let _ = is_tty(1);
        let _ = is_tty(2);
    }
}
