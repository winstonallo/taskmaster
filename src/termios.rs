use libc::*;

unsafe extern "C" {
    unsafe fn tcgetattr(fd: c_int, termios: *mut termios) -> c_int;
    unsafe fn cfmakeraw(termios: *mut termios);
}

pub fn get_current_termios() -> libc::termios {
    let mut orig: libc::termios = libc::termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; NCCS],
        c_ispeed: 0,
        c_ospeed: 0,
    };

    unsafe {
        tcgetattr(0, &raw mut orig);
    }

    orig
}

pub fn change_to_raw_mode() -> libc::termios {
    let mut orig: libc::termios = libc::termios {
        c_iflag: 0,
        c_oflag: 0,
        c_cflag: 0,
        c_lflag: 0,
        c_line: 0,
        c_cc: [0; NCCS],
        c_ispeed: 0,
        c_ospeed: 0,
    };

    unsafe {
        tcgetattr(0, &raw mut orig);
    }

    let mut raw: termios = orig;

    unsafe {
        cfmakeraw(&raw mut raw);
    }

    unsafe {
        tcsetattr(0, TCSAFLUSH, &raw);
    }

    orig
}

pub fn reset_to_termios(orig: &mut libc::termios) {
    unsafe {
        tcsetattr(0, TCSAFLUSH, orig);
    }
}
