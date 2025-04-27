use std::{env::args, process::exit, sync::atomic::AtomicU32};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use tasklib::{
    jsonrpc::{
        request::{AttachFile, RequestType},
        response::{Response, ResponseType},
    },
    shell,
};

use tasklib::jsonrpc::request::Request;

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

static ID_COUNTER: AtomicU32 = AtomicU32::new(1);

fn build_request_reload() -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_reload())
}

fn build_request_halt() -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_halt())
}

fn build_request_status() -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_status())
}

fn build_request_status_single(name: &str) -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_status_single(name))
}

fn build_request_start(name: &str) -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_start(name))
}

fn build_request_restart(name: &str) -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_restart(name))
}

fn build_request_stop(name: &str) -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_stop(name))
}

fn build_request_attach(name: &str, to: &str) -> Request {
    Request::new(
        ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        RequestType::new_attach(name, if to == "stdout" { AttachFile::StdOut } else { AttachFile::StdErr }),
    )
}

fn build_request(arguments: &Vec<&str>) -> Result<Request, &'static str> {
    let method = *arguments.first().unwrap();

    let request = match method {
        "status" => {
            if arguments.len() == 2 {
                build_request_status_single(arguments[1])
            } else if arguments.len() == 1 {
                build_request_status()
            } else {
                return Err("usage: status OR status PROCESS_NAME\n");
            }
        }
        "start" => {
            if arguments.len() == 2 {
                build_request_start(arguments[1])
            } else {
                return Err("usage: start PROCESS_NAME\n");
            }
        }
        "restart" => {
            if arguments.len() == 2 {
                build_request_restart(arguments[1])
            } else {
                return Err("usage: restart PROCESS_NAME\n");
            }
        }
        "stop" => {
            if arguments.len() == 2 {
                build_request_stop(arguments[1])
            } else {
                return Err("usage: stop PROCESS_NAME\n");
            }
        }
        "reload" => {
            if arguments.len() == 1 {
                build_request_reload()
            } else {
                return Err("usage: reload\n");
            }
        }
        "halt" => {
            if arguments.len() == 1 {
                build_request_halt()
            } else {
                return Err("usage: halt\n");
            }
        }
        "attach" => {
            if arguments.len() == 3 && ["stdout", "stderr"].contains(&arguments[2]) {
                build_request_attach(arguments[1], arguments[2])
            } else {
                return Err("usage: attach PROCESS_NAME {stdout | stderr}\n");
            }
        }
        "exit" => exit(0),
        _ => {
            return Err("method not implemented\n");
        }
    };

    Ok(request)
}

async fn attach(name: &str, socket_path: &str, to: &str) -> String {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<()>(1);
    let tx_clone = tx.clone();

    tokio::spawn(async move {
        if (tokio::signal::ctrl_c().await).is_ok() {
            let _ = tx_clone.send(()).await;
        }
    });

    let stream = match UnixStream::connect(socket_path).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("could not establish connection on attach socket at path {socket_path}: {e}");
            return "".to_string();
        }
    };

    let mut reader = BufReader::new(stream);

    loop {
        tokio::select! {
            read_result = read_from_stream(&mut reader) => {
                match read_result {
                    Ok(data) => print_raw_mode(&format!("[{name}:{to}]: {data}")),
                    Err(e) => {
                        eprintln!("attach (process: {name}): {e}");
                        break;
                    }
                }
            },
            _ = rx.recv() => {
                print_raw_mode(&format!("[{name}:{to}]: detached\n"));
                break;
            }
        }
    }
    "".to_string()
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
                StatusSingle(item) => format!("{}: {}\n", item.name(), item.state()),
                Start(name) => format!("starting: {name}\n"),
                Stop(name) => format!("stopping: {name}\n"),
                Restart(name) => format!("restarting: {name}\n"),
                Reload => "reloading configuration\n".to_string(),
                Halt => "shutting down taskmaster\n".to_string(),
                Attach { name, socketpath, to } => attach(name, socketpath, to).await,
            }
        }
        ResponseType::Error(err) => format!("{}\n", err.message),
    }
}

async fn handle_input(input: String) -> Result<String, String> {
    let arguments: Vec<&str> = input.split_ascii_whitespace().collect();
    if arguments.is_empty() {
        return Err("no non white space input given\n".to_owned());
    }

    let mut unix_stream: UnixStream = match UnixStream::connect(SOCKET_PATH).await {
        Ok(s) => s,
        Err(e) => return Err(format!("couldn't establish socket connection: {e}\n")),
    };

    let request = match build_request(&arguments) {
        Ok(request) => request,
        Err(e) => return Err(e.to_owned()),
    };

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

    Ok(response_to_str(&response).await)
}

async fn docker(args: Vec<String>) {
    let msg = match handle_input(args.join(" ")).await {
        Ok(s) => s,
        Err(s) => s,
    };
    print!("{msg}");
}

fn print_raw_mode(string: &str) {
    let mut raw_new_line = String::new();
    raw_new_line.push('\n');
    raw_new_line.push(13 as char);

    let string = string.replace('\n', &raw_new_line);
    print!("{string}")
}

async fn shell() {
    let mut shell = shell::Shell::new("taskshell> ");
    while let Some(line) = shell.next_line() {
        if line.trim() == "exit" {
            break;
        }
        let msg = match handle_input(line).await {
            Ok(s) => s,
            Err(s) => s,
        };
        print_raw_mode(&msg);
    }
}

#[tokio::main]
async fn main() {
    let args = args();
    let mut args: Vec<String> = args.collect();

    args.remove(0);

    match args.len() {
        0 => shell().await,
        _ => docker(args).await,
    };
}
