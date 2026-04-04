use std::io;

/// Safe wrapper for `getppid()` - returns parent process `ID`
pub fn get_parent_pid() -> io::Result<u32> {
    let ppid = unsafe { libc::getppid() };
    if ppid < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(ppid.try_into().unwrap())
    }
}

/// Safe wrapper for `getuid()` - returns user `ID`
pub fn get_user_id() -> io::Result<u32> {
    let uid = unsafe { libc::getuid() };
    Ok(uid)
}

/// Safe wrapper for `isatty()` - checks if file descriptor is a terminal
#[must_use]
pub fn is_tty(fd: i32) -> bool {
    unsafe { libc::isatty(fd) == 1 }
}
