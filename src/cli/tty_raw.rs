//! Raw mode makes it so demo quits on q [Enter]

#[cfg(unix)]
pub fn enter_raw_mode() -> std::io::Result<()> {
    use std::{
        io,
        mem::zeroed,
        os::raw::{c_int, c_uint},
        os::unix::io::AsRawFd,
    };

    // mini-termios – only bits we need
    #[repr(C)]
    #[allow(non_camel_case_types)]
    struct termios {
        c_iflag: c_uint,
        c_oflag: c_uint,
        c_cflag: c_uint,
        c_lflag: c_uint,
        c_line: u8,
        c_cc: [u8; 32],
        c_ispeed: c_uint,
        c_ospeed: c_uint,
    }

    unsafe extern "C" {
        fn tcgetattr(fd: c_int, termios_p: *mut termios) -> c_int;
        fn tcsetattr(fd: c_int, actions: c_int, termios_p: *const termios) -> c_int;
    }

    const TCSANOW: c_int = 0;
    const ICANON: c_uint = 0o0000002;
    const ECHO: c_uint = 0o0000010;
    const ONLCR: c_uint = 0o0000004;

    unsafe {
        let fd = std::io::stdin().as_raw_fd();
        let mut t: termios = zeroed();
        if tcgetattr(fd, &mut t) != 0 {
            return Err(io::Error::last_os_error());
        }
        t.c_lflag &= !(ICANON | ECHO);
        t.c_oflag &= !ONLCR;
        if tcsetattr(fd, TCSANOW, &t) != 0 {
            return Err(io::Error::last_os_error());
        }
    }
    Ok(())
}

#[cfg(not(unix))]
pub fn enter_raw_mode() -> std::io::Result<()> {
    Ok(())
}
