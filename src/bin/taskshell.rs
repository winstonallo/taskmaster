use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
    sync::atomic::AtomicU32,
};

use tasklib::jsonrpc::{
    request::RequestType,
    response::{Response, ResponseType},
};

use tasklib::jsonrpc::request::Request;

fn read_from_stream(unix_stream: &mut UnixStream) -> Result<String, String> {
    let mut buf = String::new();

    unix_stream.read_to_string(&mut buf).map_err(|e| format!("{}", e))?;

    Ok(buf)
}

fn write_request(unix_stream: &mut UnixStream, request: &[u8]) -> Result<(), String> {
    unix_stream.write(request).map_err(|e| format!("{}", e))?;
    unix_stream.shutdown(std::net::Shutdown::Write).map_err(|e| format!("{}", e))?;

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
    Request::new(
        ID_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        RequestType::new_status_single(name),
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

const SOCKET_PATH: &str = "/tmp/.taskmaster.sock";

fn main() {
    loop {
        let mut unix_stream: UnixStream = match UnixStream::connect(SOCKET_PATH) {
            Ok(s) => s,
            Err(e) => {
                println!("couldn't establish socket connection: {}", e);
                return;
            }
        };

        print!("taskshell> ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();

        let arguments: Vec<&str> = line.split_ascii_whitespace().collect();
        if arguments.len() < 1 {
            continue;
        }

        let method = *arguments.get(0).unwrap();

        let request = match method {
            "status" => {
                if arguments.len() == 2 {
                    build_request_status_single(arguments[1])
                } else if arguments.len() == 1 {
                    build_request_status()
                } else {
                    println!("usage: status OR status PROCESS_NAME");
                    continue;
                }
            }
            "start" => {
                if arguments.len() == 2 {
                    build_request_start(arguments[1])
                } else {
                    println!("usage: start PROCESS_NAME");
                    continue;
                }
            }
            "restart" => {
                if arguments.len() == 2 {
                    build_request_restart(arguments[1])
                } else {
                    println!("usage: restart PROCESS_NAME");
                    continue;
                }
            }
            "stop" => {
                if arguments.len() == 2 {
                    build_request_stop(arguments[1])
                } else {
                    println!("usage: stop PROCESS_NAME");
                    continue;
                }
            }
            "reload" => {
                if arguments.len() == 1 {
                    build_request_reload()
                } else {
                    println!("usage: reload");
                    continue;
                }
            }
            "halt" => {
                if arguments.len() == 1 {
                    build_request_halt()
                } else {
                    println!("usage: halt");
                    continue;
                }
            }
            "exit" => return,
            _ => {
                println!("method not implemented");
                continue;
            }
        };

        let request = serde_json::to_string(&request).unwrap();

        let _ = write_request(&mut unix_stream, request.as_bytes());

        let response = match read_from_stream(&mut unix_stream) {
            Ok(resp) => resp,
            Err(e) => {
                println!("error while reading socket: {}", e);
                continue;
            }
        };

        let response = match serde_json::from_str::<Response>(&response) {
            Ok(resp) => resp,
            Err(e) => {
                println!("message: {}", response);
                continue;
            }
        };

        match response.response_type() {
            ResponseType::Result(res) => {
                use tasklib::jsonrpc::response::ResponseResult::*;
                let str = match res {
                    Status(items) => {
                        let mut str = String::new();
                        for short_process in items.iter() {
                            str.push_str(&format!("Name: {}\t State: {}\n", short_process.name(), short_process.state()));
                        }
                        str
                    }
                    StatusSingle(short_process) => format!("Name: {}, State: {}\n", short_process.name(), short_process.state()),
                    Start(name) => format!("staring: {}\n", name),
                    Stop(name) => format!("stopping: {}\n", name),
                    Restart(name) => format!("restarting: {}\n", name),
                    Reload => "reloading configuration\n".to_owned(),
                    Halt => "shutting down taskmaster\n".to_owned(),
                };

                print!("response from daemon: \n{}", str)
            }
            ResponseType::Error(err) => {
                println!("{}", err.message)
            }
        }
    }
}
