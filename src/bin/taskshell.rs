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
    shell::{
        self,
        args::{Args, Command, EngineSubcommand},
    },
};

use tasklib::jsonrpc::request::Request;

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

fn build_request_status(name: &Option<String>) -> Request {
    Request::new(
        ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        match name {
            Some(name) => RequestType::new_status_single(&name),
            None => RequestType::new_status(),
        },
    )
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

fn build_request_attach(name: &str, to: &AttachFile) -> Request {
    Request::new(ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed), RequestType::new_attach(name, to))
}

enum BuildRequestResult {
    Request(Request),
    NoOp(String),
    Exit(i32),
}

fn build_request(command: &Command) -> Result<BuildRequestResult, String> {
    let request = match command {
        Command::Status { process } => build_request_status(process),
        Command::Start { process } => build_request_start(&process),
        Command::Restart { process } => build_request_restart(&process),
        Command::Stop { process } => build_request_stop(&process),
        Command::Attach { process, fd } => build_request_attach(&process, fd),
        Command::Reload => build_request_reload(),
        Command::Exit => return Ok(BuildRequestResult::Exit(0)),
        Command::Engine { subcommand } => match subcommand {
            EngineSubcommand::Start => {
                /* start engine */
                return Ok(BuildRequestResult::NoOp("taskmaster engine successfully started".to_string()));
            }
            EngineSubcommand::Stop => build_request_halt(),
        },
    };

    Ok(BuildRequestResult::Request(request))
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

async fn handle_input(input: Vec<String>) -> Result<String, String> {
    let arguments = Args::try_from(input)?;

    let mut unix_stream: UnixStream = match UnixStream::connect(arguments.socketpath()).await {
        Ok(s) => s,
        Err(e) => return Err(format!("couldn't establish socket connection: {e}\n")),
    };

    let request = match build_request(arguments.command()) {
        Ok(res) => match res {
            BuildRequestResult::Exit(code) => exit(code),
            BuildRequestResult::NoOp(msg) => return Ok(msg),
            BuildRequestResult::Request(request) => request,
        },
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
    let msg = match handle_input(args).await {
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
        let msg = match handle_input(
            line.split_ascii_whitespace()
                .into_iter()
                .map(String::from)
                .collect::<Vec<String>>(),
        )
        .await
        {
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
