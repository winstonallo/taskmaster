use std::{
    io::{Read, Write, stdout},
    sync::atomic::AtomicU32,
    time::Duration,
};

use tasklib::{
    jsonrpc::{
        request::{Request, RequestType},
        response::{Response, ResponseResult, ResponseType}, short_process::ShortProcess,
    },
    termios::{change_to_raw_mode, reset_to_termios},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    join,
    net::UnixStream,
    select,
    time::sleep,
};


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

//async fn response_status_to_str(response: &Response) -> String {


    //let mut str = String::new();
    //for short_process in processes.iter() {
        //str.push_str(&format!("{}: {}\n", short_process.name(), short_process.state()));
    //}
    //str
//}

static ID_COUNTER: AtomicU32 = AtomicU32::new(1);

const SOCKET_PATH: &str = "/tmp/.taskmaster.sock";

#[derive(Debug)]
enum Content {
    Processes(Vec<ShortProcess>),
    Error(String),
    Empty
}

struct TaskBoard {
    buf: [u8; 1],
    content: Content,
    scrolled_lines_down: usize,
    terminal_height: usize,
    terminal_witdh: usize,
    command_started: bool,
    command_arrow: bool,
}

impl TaskBoard {
    pub fn new() -> Self {
        Self {
            buf: [0; 1],
            scrolled_lines_down: 0,
            terminal_height: 0,
            terminal_witdh: 0,
            command_arrow: false,
            command_started: false,
            content: Content::Empty
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
                   if let Err(_) = self.handle_keyboard_entry(buf[0]) {
                       break;
                    }
                },
                _ = sleep(Duration::from_millis(500)) => {
                    self.get_status().await
                }
            }
            println!("{:?}", self.content)
        }
        reset_to_termios(orig_termios);
    }
    
    async fn get_status(&mut self) {
        let response = match Self::call_status().await {
            Ok(r) => r,
            Err(e) => {
                self.content = Content::Error(e);
                return
            }
        };

        let result = match response.response_type() {
            ResponseType::Result(result) => result,
            ResponseType::Error(err) => { self.content = Content::Error(format!("{}\n", err.message));
        return}
        };
    
        self.content = match result {
            ResponseResult::Status(processes) => Content::Processes(processes.to_vec()),
            _ => panic!("this should only get Status responses nothing else"),
        };
    }
    
    async fn call_status() -> Result<Response, String> {
        let mut unix_stream: UnixStream = match UnixStream::connect(SOCKET_PATH).await {
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
            3 | 4 | b'q' => return Err(()),
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
        self.terminal_witdh = w;
        self.terminal_height = h;
    }

    fn clear_screen(&self) {
        print!("{esc}[2J{esc}[{};{}H", self.terminal_witdh, self.terminal_height, esc = 27 as char);
    }
}

fn move_cursor_up() {
    print!("\x1b[1A");
}

fn clear_line(width: usize) {
    print!("{}", 13 as char);
    for _ in 0..width {
        print!(" ");
    }
    print!("{}", 13 as char);
    stdout().flush();
}

#[tokio::main]
async fn main() {
    let mut taskboard = TaskBoard::new();
    taskboard.run().await;
}

//let orig_termios = change_to_raw_mode();

//let mut stdin = tokio::io::stdin();
//loop {
//select! {
//Ok(bytes_read) = stdin.read_exact(&mut buf) => {
//if bytes_read == 0 {
//continue;
//}

//let c: u8 = buf[0];

//match c {
//b'q' => {
//command_mode = false;
//let mut s = String::from("\n");
//s.push(13 as char);
//print!("{}", s);
//return;
//}
//b'[' => {
//command_mode = true;
//}
//b'A' => {
//if command_mode {
//scrolled_lines_down = scrolled_lines_down.saturating_sub(1);
//}
//command_mode = false;
//}
//b'B' => {
//if command_mode {

//scrolled_lines_down = scrolled_lines_down.saturating_add(1);
//}
//command_mode = false;
//}
//_=> {
//command_mode = false;
//}
//}
//},
//_ = sleep(Duration::from_millis(5)) => {

//let (w, mut h) = match  term_size::dimensions() {
//Some(x) => x,
//None => panic!("couldn't get terminal width and height")
//};

//h -= 1;
//for _ in 0..h {
//clear_line(w);
//move_cursor_up();
//clear_line(w);
//}

//if scrolled_lines_down > h {
//scrolled_lines_down = h;
//}

//let res = handle().await;

//let response = match res {
//Ok(str) => str,
//Err(e) => {
//println!("{}", &e);
//return;
//}
//};

//let response_result = match response.response_type() {
//ResponseType::Result(s) => s,
//_ => return,
//};

//let processes = match response_result {
//ResponseResult::Status(s) => s,
//_ => return,
//};

//let mut lines = Vec::new();

//for p in processes.iter() {
//lines.push(format!("State: {}, Name: {}",  p.state(),p.name()));
//}

//lines.sort();

//let max_line = lines.iter().max_by(|a, b| a.len().cmp(&b.len()));

//let max_line = match max_line {
//Some(l) => l,
//None => return
//};

//let max_line_length = max_line.len();

//if max_line_length > w {
//println!("terminal not big enough to display processes | {} width needed {} given",max_line_length, w );
//return ;
//}

//if h >= lines.len() {
//scrolled_lines_down = 0;
//}

//for (_, line) in lines.iter_mut().enumerate().skip(scrolled_lines_down).take(h) {
//line.push(13 as char);
//println!("{}", line);
//}

//if h >= lines.iter().skip(scrolled_lines_down).len() {
//for _ in 0..(h-lines.iter().skip(scrolled_lines_down).len()) {
//let mut s = String::from("\n");
//s.push(13 as char);
//print!("{}", s)

//}
//}

//print!("Press 'q' to quit!");
//stdout().flush();
//}
//}
//}
//}

