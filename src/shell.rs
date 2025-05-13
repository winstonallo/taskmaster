use std::io::{self, Read, Write};

extern crate libc;

unsafe extern "C" {
    fn raw_mod();
}
use libc::{TIOCGWINSZ, ioctl, winsize};
use std::io::stdout;
use std::mem::MaybeUninit;
use std::os::unix::io::AsRawFd;

pub mod args;

fn get_terminal_size() -> io::Result<(u16, u16)> {
    unsafe {
        let mut ws: winsize = MaybeUninit::zeroed().assume_init();
        if ioctl(stdout().as_raw_fd(), TIOCGWINSZ, &mut ws) == 0 {
            Ok((ws.ws_col, ws.ws_row))
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

pub struct Shell {
    prompt: String,
    line: String,
    width: usize,
    history: Vec<String>,
}

impl Shell {
    pub fn new(prompt: &str) -> Self {
        let (width, _) = match get_terminal_size() {
            Ok((width, height)) => (width, height),
            Err(e) => panic!("Failed to get terminal size: {e}"),
        };
        unsafe {
            raw_mod();
        }
        Self {
            prompt: prompt.to_owned(),
            line: String::new(),
            width: width as usize,
            history: vec![],
        }
    }

    fn clear_string(&self, line: &str) {
        self.clear_line();
        if !line.is_empty() {
            let wrapping_rows = (line.len() - 1) / self.width;
            for _ in 0..wrapping_rows {
                Self::move_cursor_up();
                self.clear_line();
            }
        }
    }

    pub fn next_line(&mut self) -> Option<String> {
        let mut waiting_for_arrow_command = false;
        let mut history_index = 0;

        self.line.push_str(&self.prompt);
        print!("{}", self.line);
        let _ = io::stdout().flush();

        let mut buf: [u8; 1] = [0; 1];
        while let Ok(bytes_read) = io::stdin().lock().read(&mut buf) {
            if bytes_read == 0 {
                continue;
            }

            let c = buf[0];

            if history_index == 0 {
                self.clear_string(&self.line);
            } else {
                self.clear_string(&self.history[self.history.len() - history_index]);
            }

            match (c, waiting_for_arrow_command) {
                (b'A', true) => {
                    history_index += 1;
                    if history_index > self.history.len() {
                        history_index = self.history.len()
                    }
                }
                (b'B', true) => {
                    history_index = history_index.saturating_sub(1);
                }
                (b'[', false) => waiting_for_arrow_command = true,
                _ => {
                    waiting_for_arrow_command = false;
                    match c {
                        3 => {
                            break;
                        }
                        91 => {
                            waiting_for_arrow_command = true;
                            continue;
                        }

                        32..127 => {
                            if history_index != 0 {
                                self.line = self.history[self.history.len() - history_index].clone();
                                history_index = 0;
                            }
                            self.line.push(c as char);
                        }
                        13 => {
                            if history_index != 0 {
                                self.line = self.history[self.history.len() - history_index].clone();
                            }
                            if !self.line.trim().is_empty() {
                                self.history.push(self.line.clone());
                            }
                            print!("{}", self.line);
                            let _ = io::stdout().flush();
                            println!();
                            self.clear_line();
                            let mut res = self.line.clone();
                            self.line.clear();
                            self.line = String::new();
                            res.push('\n');
                            res = res[self.prompt.len()..].to_owned();
                            return Some(res);
                        }
                        127 => {
                            if history_index != 0 {
                                self.line = self.history[self.history.len() - history_index].clone();
                                history_index = 0;
                            }
                            if self.line.len() > self.prompt.len() {
                                self.line.pop();
                            }
                        }
                        _ => {}
                    }
                }
            }

            if history_index == 0 {
                print!("{}", self.line)
            } else {
                print!("{}", &self.history[self.history.len() - history_index]);
            }

            let _ = io::stdout().flush();
        }
        None
    }

    fn move_cursor_up() {
        print!("\x1b[1A");
    }

    fn clear_line(&self) {
        print!("{}", 13 as char);
        for _ in 0..self.width {
            print!(" ");
        }
        print!("{}", 13 as char);
    }
}
