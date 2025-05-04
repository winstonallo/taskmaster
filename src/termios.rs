use std::io::{self, Read};

use libc::*;

unsafe extern "C" {
    unsafe fn tcgetattr(fd: c_int, termios: *mut termios) -> c_int;
    unsafe fn cfmakeraw(termios: *mut termios);
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

pub fn reset_to_termios(orig: libc::termios) {

    unsafe {
        tcsetattr(0, TCSAFLUSH, &orig);
    }
}

fn main() {

    let orig = change_to_raw_mode();
    let mut command_started: bool = false;

    let mut command_arrow: bool = false;

    let mut buf: [u8; 1] = [0; 1];
    while let Ok(bytes_read) = io::stdin().lock().read(&mut buf) {
        let c: u8 = buf[0];


        match c {
            3 | 4 => break,
            27 => command_started = true,
            91 => if command_started { command_arrow  = true}

            _ =>{ 
                if command_arrow {
                    match c {
                        b'A' => println!("up"),
                        b'B' => println!("down"),
                        _ => {}
                    }

                    match c {
                        b'A' | b'B' => continue,
                        _ => {}
                    }
                } 
                    command_arrow = false;
                    command_started = false;
                println!("{} | {}", buf[0], bytes_read);}
        }
    }

    reset_to_termios(orig);
}