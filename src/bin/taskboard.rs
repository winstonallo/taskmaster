use std::{env, io::Write, process::exit, sync::atomic::AtomicU32, time::Duration};

use tasklib::{
    conf::defaults::dflt_socketpath,
    jsonrpc::{
        request::{Request, RequestType},
        response::{Response, ResponseResult, ResponseType},
        short_process::ShortProcess,
    },
    termios::{change_to_raw_mode, reset_to_termios},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    select,
    time::sleep,
};

static ID_COUNTER: AtomicU32 = AtomicU32::new(1);

#[derive(Clone, Debug)]
enum Content {
    Processes(Vec<ShortProcess>),
    Error(String),
    Empty,
}

struct TaskBoard {
    content: Content,
    scrolled_lines_down: usize,
    terminal_height: usize,
    terminal_width: usize,
    command_started: bool,
    command_arrow: bool,
    socketpath: String,
}

impl TaskBoard {
    pub fn new(socketpath: &str) -> Self {
        Self {
            scrolled_lines_down: 0,
            terminal_height: 0,
            terminal_width: 0,
            command_arrow: false,
            command_started: false,
            content: Content::Empty,
            socketpath: socketpath.to_owned(),
        }
    }

    pub async fn run(&mut self) {
        self.load_terminal_dimnsions();
        self.clear_screen();

        let orig_termios = change_to_raw_mode();

        let mut buf: [u8; 1] = [0; 1];

        let mut stdin = tokio::io::stdin();
        loop {
            select! {
               Ok(_) = stdin.read_exact(&mut buf) => {
                   if self.handle_keyboard_entry(buf[0]).is_err() {
                       break;
                    }
                },
                _ = sleep(Duration::from_millis(500)) => {
                    self.get_status_from_daemon().await
                }
            }
            self.clear_screen();
            self.display_content();
        }
        self.clear_screen();
        reset_to_termios(orig_termios);
    }

    fn display_content(&mut self) {
        self.clear_screen();

        match self.content.clone() {
            Content::Empty => {}
            Content::Error(mut e) => {
                e.push(13 as char);
                println!("{}", e);
            }
            Content::Processes(processes) => {
                self.load_terminal_dimnsions();
                let height = self.terminal_height - 1;
                if height >= processes.len() {
                    self.scrolled_lines_down = 0;
                }

                if processes.len() < (height + self.scrolled_lines_down) {
                    self.scrolled_lines_down = processes.len().saturating_sub(height);
                }

                let mut lines = Vec::new();

                for p in processes.iter() {
                    lines.push(format!("State: {}, Name: {}", p.state(), p.name()));
                }

                lines.sort();

                let max_line = lines.iter().max_by(|a, b| a.len().cmp(&b.len()));

                let max_line = match max_line {
                    Some(l) => l,
                    None => return,
                };

                let max_line_length = max_line.len();

                if max_line_length > self.terminal_width {
                    println!("terminal not big enough to display processes | {} width needed {} given", max_line_length, self.terminal_width);
                    return;
                }

                for (_, line) in lines.iter_mut().enumerate().skip(self.scrolled_lines_down).take(height) {
                    line.push(13 as char);
                    println!("{}", line);
                }
            }
        }
        print!("Press 'q' to quit - use arrow keys to move the list");
        let _ = std::io::stdout().flush();
    }

    async fn get_status_from_daemon(&mut self) {
        let response = match self.make_json_rpc_status_request().await {
            Ok(r) => r,
            Err(e) => {
                self.content = Content::Error(e);
                return;
            }
        };

        let result = match response.response_type() {
            ResponseType::Result(result) => result,
            ResponseType::Error(err) => {
                self.content = Content::Error(format!("{}\n", err.message));
                return;
            }
        };

        self.content = match result {
            ResponseResult::Status(processes) => Content::Processes(processes.to_vec()),
            _ => panic!("this should only get Status responses nothing else"),
        };
    }

    async fn make_json_rpc_status_request(&self) -> Result<Response, String> {
        let mut unix_stream: UnixStream = match UnixStream::connect(&self.socketpath).await {
            Ok(s) => s,
            Err(e) => return Err(format!("couldn't establish socket connection: {e}\n")),
        };
        let request = Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_status());

        let request_str = serde_json::to_string(&request).unwrap(); // unwrap because this should never fail

        if let Err(e) = write_to_stream(&mut unix_stream, request_str.as_bytes()).await {
            return Err(format!("error while writing request: {e}\n"));
        }

        let mut reader = BufReader::new(unix_stream);
        let response = match read_from_stream(&mut reader).await {
            Ok(resp) => resp,
            Err(e) => return Err(format!("error while reading socket: {e}\n")),
        };

        let response = match serde_json::from_str::<Response>(&response) {
            Ok(resp) => resp,
            Err(_) => return Err(format!("non json_rpc formatted message: {response}\n")),
        };

        Ok(response)
    }

    fn handle_keyboard_entry(&mut self, entry: u8) -> Result<(), ()> {
        match entry {
            b'q' => return Err(()),
            27 => self.command_started = true,
            91 => {
                if self.command_started {
                    self.command_arrow = true
                }
            }
            b'A' => {
                self.scrolled_lines_down = self.scrolled_lines_down.saturating_sub(1);
                self.command_arrow = false;
                self.command_started = false;
            }
            b'B' => {
                self.scrolled_lines_down = self.scrolled_lines_down.saturating_add(1);
                self.command_arrow = false;
                self.command_started = false;
            }
            _ => {}
        }
        Ok(())
    }

    fn load_terminal_dimnsions(&mut self) {
        let (w, h) = match term_size::dimensions() {
            Some(x) => x,
            None => panic!("couldn't get terminal width and height"),
        };
        self.terminal_width = w;
        self.terminal_height = h;
    }

    fn clear_line(width: usize) {
        let c: char = 13 as char;
        print!("{c}");
        for _ in 0..width {
            print!(" ")
        }
        print!("{c}");
    }

    fn clear_screen(&self) {
        for _ in 0..self.terminal_height {
            Self::clear_line(self.terminal_width);
            move_cursor_up();
            Self::clear_line(self.terminal_width);
        }
    }
}

fn move_cursor_up() {
    print!("\x1b[1A");
}

#[derive(Debug)]
struct Args {
    socketpath: String,
}

impl TryFrom<Vec<String>> for Args {
    type Error = String;

    fn try_from(value: Vec<String>) -> Result<Self, Self::Error> {
        if value.is_empty() {
            return Ok(Self {
                socketpath: match env::var("TASKMASTER_SOCKETPATH") {
                    Ok(path) => path,
                    Err(_) => dflt_socketpath(),
                },
            });
        }

        if value.len() != 2 {
            return Err("usage: --socketpath|-s PATH".to_string());
        }

        if value[0] != "-s" && value[0] != "--socketpath" {
            return Err(format!("unexpected option: {}\nusage: --socketpath|-s PATH", value[0]));
        }

        Ok(Self {
            socketpath: value[1].to_owned(),
        })
    }
}

#[tokio::main]
async fn main() {
    let mut args: Vec<String> = env::args().collect::<Vec<String>>();
    args.remove(0);

    let args = match Args::try_from(args) {
        Ok(args) => args,
        Err(e) => {
            eprintln!("{e}");
            exit(1);
        }
    };

    let mut taskboard = TaskBoard::new(&args.socketpath);
    taskboard.run().await;
}

async fn read_from_stream<R>(reader: &mut BufReader<R>) -> Result<String, String>
where
    R: AsyncRead + Unpin,
{
    let mut buf = String::new();
    let bytes_read = reader.read_line(&mut buf).await.map_err(|e| e.to_string())?;

    let s = String::from_utf8_lossy(&buf.as_bytes()[0..bytes_read]).to_string();
    Ok(s)
}

async fn write_to_stream(unix_stream: &mut UnixStream, msg: &[u8]) -> Result<(), String> {
    unix_stream.write_all(msg).await.map_err(|e| e.to_string())?;
    unix_stream.shutdown().await.map_err(|e| e.to_string())?;

    Ok(())
}
