use std::{
    env::args,
    io::{Read, Write},
    os::unix::net::UnixStream,
    process::exit,
    sync::atomic::AtomicU32,
};

use tasklib::{
    jsonrpc::{
        request::RequestType,
        response::{Response, ResponseType},
    },
    shell,
};

use tasklib::jsonrpc::request::Request;

const SOCKET_PATH: &str = "/tmp/.taskmaster.sock";

fn read_from_stream(unix_stream: &mut UnixStream) -> Result<String, String> {
    let mut buf = String::new();

    unix_stream.read_to_string(&mut buf).map_err(|e| format!("{e}"))?;

    Ok(buf)
}

fn write_request(unix_stream: &mut UnixStream, request: &[u8]) -> Result<(), String> {
    unix_stream.write(request).map_err(|e| format!("{e}"))?;
    unix_stream.shutdown(std::net::Shutdown::Write).map_err(|e| format!("{e}"))?;

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
        "exit" => exit(0),
        _ => {
            return Err("method not implemented\n");
        }
    };

    Ok(request)
}

fn response_to_str(response: Response) -> String {
    match response.response_type() {
        ResponseType::Result(res) => {
            use tasklib::jsonrpc::response::ResponseResult::*;
            match res {
                Status(items) => {
                    let mut str = String::new();
                    for short_process in items.iter() {
                        str.push_str(&format!("Name: {}\t State: {}\n", short_process.name(), short_process.state()));
                    }
                    str
                }
                StatusSingle(short_process) => format!("Name: {}, State: {}\n", short_process.name(), short_process.state()),
                Start(name) => format!("starting: {name}\n"),
                Stop(name) => format!("stopping: {name}\n"),
                Restart(name) => format!("restarting: {name}\n"),
                Reload => "reloading configuration\n".to_owned(),
                Halt => "shutting down taskmaster\n".to_owned(),
            }
        }
        ResponseType::Error(err) => err.message.clone() + "\n",
    }
}

fn handle_input(input: String) -> Result<String, String> {
    let arguments: Vec<&str> = input.split_ascii_whitespace().collect();
    if arguments.is_empty() {
        return Err("no non white space input given\n".to_owned());
    }

    let mut unix_stream: UnixStream = match UnixStream::connect(SOCKET_PATH) {
        Ok(s) => s,
        Err(e) => return Err(format!("couldn't establish socket connection: {e}\n")),
    };

    let request = match build_request(&arguments) {
        Ok(request) => request,
        Err(e) => return Err(e.to_owned()),
    };

    let request = serde_json::to_string(&request).unwrap(); // unwrap because this should never fail

    if let Err(e) = write_request(&mut unix_stream, request.as_bytes()) {
        return Err(format!("error while writing request: {e}\n"));
    }

    let response = match read_from_stream(&mut unix_stream) {
        Ok(resp) => resp,
        Err(e) => return Err(format!("error while reading socket: {e}\n")),
    };

    let response = match serde_json::from_str::<Response>(&response) {
        Ok(resp) => resp,
        Err(_) => return Err(format!("non json_rpc formated message: {response}\n")),
    };

    Ok(response_to_str(response))
}

fn docker(args: Vec<String>) {
    let msg = match handle_input(args.join(" ")) {
        Ok(s) => s,
        Err(s) => s,
    };
    print!("{msg}");
}

fn print_raw_mode(string: &str) {
    let mut raw_new_line = String::new();
    raw_new_line.push(13 as char);
    raw_new_line.push('\n');

    let string = string.replace('\n', &raw_new_line);
    print!("{string}")
}

fn shell() {
    let mut shell = shell::Shell::new("taskshell> ");
    while let Some(line) = shell.next_line() {
        let msg = match handle_input(line) {
            Ok(s) => s,
            Err(s) => s,
        };
        print_raw_mode(&msg);
    }
}

fn main() {
    let args = args();
    let mut args: Vec<String> = args.collect();

    args.remove(0);

    match args.len() {
        0 => shell(),
        _ => docker(args),
    }
}
