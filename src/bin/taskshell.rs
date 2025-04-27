use std::{io::Write, process::exit, sync::atomic::AtomicU32};

use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncWriteExt, BufReader},
    net::UnixStream,
};

use tasklib::jsonrpc::{
    request::RequestType,
    response::{Response, ResponseType},
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

fn build_request_attach(name: &str) -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_attach(name))
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
                return Err("usage: status OR status PROCESS_NAME");
            }
        }
        "start" => {
            if arguments.len() == 2 {
                build_request_start(arguments[1])
            } else {
                return Err("usage: start PROCESS_NAME");
            }
        }
        "restart" => {
            if arguments.len() == 2 {
                build_request_restart(arguments[1])
            } else {
                return Err("usage: restart PROCESS_NAME");
            }
        }
        "stop" => {
            if arguments.len() == 2 {
                build_request_stop(arguments[1])
            } else {
                return Err("usage: stop PROCESS_NAME");
            }
        }
        "reload" => {
            if arguments.len() == 1 {
                build_request_reload()
            } else {
                return Err("usage: reload");
            }
        }
        "halt" => {
            if arguments.len() == 1 {
                build_request_halt()
            } else {
                return Err("usage: halt");
            }
        }
        "attach" => {
            if arguments.len() == 2 {
                build_request_attach(arguments[1])
            } else {
                return Err("usage: attach PROCESS_NAME");
            }
        }
        "exit" => exit(0),
        _ => {
            return Err("method not implemented");
        }
    };

    Ok(request)
}

async fn attach(name: &str, socket_path: &str) {
    let stream = match UnixStream::connect(socket_path).await {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("could not establish connection on attach socket at path {socket_path}: {e}");
            return;
        }
    };

    let mut reader = BufReader::new(stream);

    loop {
        match read_from_stream(&mut reader).await {
            Ok(data) => print!("{data}"),
            Err(e) => {
                eprintln!("attach: {e}");
                break;
            }
        }
    }
    println!("attaching to process {name} on socket {socket_path}");
}

async fn handle_response(response: &Response) {
    match response.response_type() {
        ResponseType::Result(res) => {
            use tasklib::jsonrpc::response::ResponseResult::*;
            match res {
                Status(items) => items.iter().for_each(|sp| println!("{}: {}", sp.name(), sp.state())),
                StatusSingle(item) => println!("{}: {}", item.name(), item.state()),
                Start(name) => println!("starting: {name}"),
                Stop(name) => println!("stopping: {name}"),
                Restart(name) => println!("restarting: {name}"),
                Reload => println!("reloading configuration"),
                Halt => println!("shutting down taskmaster\n"),
                Attach { name, socketpath } => attach(name, socketpath).await,
            };
        }
        ResponseType::Error(err) => {
            println!("{}", err.message)
        }
    }
}

#[tokio::main]
async fn main() {
    loop {
        let mut unix_stream: UnixStream = match UnixStream::connect(SOCKET_PATH).await {
            Ok(s) => s,
            Err(e) => {
                eprintln!("couldn't establish socket connection: {e}");
                return;
            }
        };

        print!("taskshell> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();

        let arguments: Vec<&str> = line.split_ascii_whitespace().collect();
        if arguments.is_empty() {
            break;
        }

        let request = match build_request(&arguments) {
            Ok(request) => request,
            Err(e) => {
                println!("{e}");
                continue;
            }
        };

        let request_str = serde_json::to_string(&request).unwrap(); // unwrap because this should never fail

        if let Err(e) = write_request(&mut unix_stream, request_str.as_bytes()).await {
            println!("error while writing request: {e}");
            continue;
        }

        let mut reader = BufReader::new(unix_stream);
        let response = match read_from_stream(&mut reader).await {
            Ok(resp) => resp,
            Err(e) => {
                println!("error while reading socket: {e}");
                continue;
            }
        };

        println!("{response}");

        let mut response = match serde_json::from_str::<Response>(&response) {
            Ok(resp) => resp,
            Err(_) => {
                println!("non json_rpc formatted message: {response}");
                continue;
            }
        };

        let response = response.set_response_result(request.request_type());

        handle_response(response).await;
    }
}
