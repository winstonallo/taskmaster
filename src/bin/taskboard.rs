use std::{
    io::{Read, Write, stdout},
    sync::atomic::AtomicU32,
    time::Duration,
};

use tasklib::jsonrpc::{
    request::{Request, RequestType},
    response::{Response, ResponseResult, ResponseType},
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWriteExt, BufReader},
    net::UnixStream,
    select,
    time::sleep,
};

static ID_COUNTER: AtomicU32 = AtomicU32::new(1);

const SOCKET_PATH: &str = "/tmp/.taskmaster.sock";
async fn read_from_stream<R>(reader: &mut BufReader<R>) -> Result<String, String>
where
    R: AsyncRead + Unpin,
{
    let mut buf = String::new();
    let bytes_read = reader.read_line(&mut buf).await.map_err(|e| e.to_string())?;

    let s = String::from_utf8_lossy(&buf.as_bytes()[0..bytes_read]).to_string();
    Ok(s)
}

async fn write_request(unix_stream: &mut UnixStream, request: &[u8]) -> Result<(), String> {
    unix_stream.write_all(request).await.map_err(|e| e.to_string())?;
    unix_stream.shutdown().await.map_err(|e| e.to_string())?;

    Ok(())
}
async fn handle() -> Result<Response, String> {
    let mut unix_stream: UnixStream = match UnixStream::connect(SOCKET_PATH).await {
        Ok(s) => s,
        Err(e) => return Err(format!("couldn't establish socket connection: {e}\n")),
    };

    let request = build_request_status();

    let request_str = serde_json::to_string(&request).unwrap(); // unwrap because this should never fail

    if let Err(e) = write_request(&mut unix_stream, request_str.as_bytes()).await {
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
async fn response_to_str(response: &Response) -> String {
    match response.response_type() {
        ResponseType::Result(res) => {
            use tasklib::jsonrpc::response::ResponseResult::*;
            match res {
                Status(items) => {
                    let mut str = String::new();
                    for short_process in items.iter() {
                        str.push_str(&format!("{}: {}\n", short_process.name(), short_process.state()));
                    }
                    str
                }
                _ => panic!("this should only get Status responses nothing else"),
            }
        }
        ResponseType::Error(err) => format!("{}\n", err.message),
    }
}

unsafe extern "C" {
    fn raw_mod();
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
    let (w, h) = match term_size::dimensions() {
        Some(x) => x,
        None => panic!("couldn't get terminal width and height"),
    };
    print!("{esc}[2J{esc}[{w};{h}H", esc = 27 as char);
    unsafe {
        raw_mod();
    }

    let mut buf: [u8; 1] = [0; 1];
    let mut scrolled_lines_down: usize = 0;
    let mut command_mode = false;

    let mut stdin = tokio::io::stdin();
    loop {
        select! {
           Ok(bytes_read) = stdin.read_exact(&mut buf) => {
                if bytes_read == 0 {
                    continue;
                }

                let c: u8 = buf[0];

                match c {
                    b'q' => {
                        command_mode = false;
                        let mut s = String::from("\n");
                        s.push(13 as char);
                        print!("{}", s);
                        return;
                    }
                    b'[' => {
                        command_mode = true;
                    }
                    b'A' => {
                        if command_mode {
                            scrolled_lines_down = scrolled_lines_down.saturating_sub(1);
                        }
                        command_mode = false;
                    }
                    b'B' => {
                        if command_mode {

                            scrolled_lines_down = scrolled_lines_down.saturating_add(1);
                        }
                        command_mode = false;
                    }
                    _=> {
                        command_mode = false;
                    }
                }
            },
            _ = sleep(Duration::from_millis(5)) => {

            let (w, mut h) = match  term_size::dimensions() {
                Some(x) => x,
                None => panic!("couldn't get terminal width and height")
            };

            h -= 1;
            for _ in 0..h {
                clear_line(w);
                move_cursor_up();
                clear_line(w);
            }

            if scrolled_lines_down > h {
                scrolled_lines_down = h;
            }


            let res = handle().await;

            let response = match res {
                Ok(str) => str,
                Err(e) => {
                    println!("{}", &e);
                    return;
                }
            };

            let response_result = match response.response_type() {
                ResponseType::Result(s) => s,
                _ => return,
            };

            let processes = match response_result {
                ResponseResult::Status(s) => s,
                _ => return,
            };

            let mut lines = Vec::new();

            for p in processes.iter() {
                lines.push(format!("State: {}, Name: {}",  p.state(),p.name()));
            }

            lines.sort();

            let max_line = lines.iter().max_by(|a, b| a.len().cmp(&b.len()));

            let max_line = match max_line {
                Some(l) => l,
                None => return
            };

            let max_line_length = max_line.len();

            if max_line_length > w {
                println!("terminal not big enough to display processes | {} width needed {} given",max_line_length, w );
                return ;
            }

            if h >= lines.len() {
                scrolled_lines_down = 0;
            }

            for (_, line) in lines.iter_mut().enumerate().skip(scrolled_lines_down).take(h) {
                line.push(13 as char);
                println!("{}", line);
            }

            if h >= lines.iter().skip(scrolled_lines_down).len() {
                for _ in 0..(h-lines.iter().skip(scrolled_lines_down).len()) {
                    let mut s = String::from("\n");
                    s.push(13 as char);
                    print!("{}", s)

                }
            }

            print!("Press 'q' to quit!");
            stdout().flush();
            }
        }
    }
}

fn build_request_status() -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_status())
}
