use core::panic;
use std::{
    io::{Read, Write, stdout},
    os::unix::net::UnixStream,
};

use libc::stat;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tasklib::jsonrpc::{JsonRPCError, JsonRPCRequest, JsonRPCResponse};

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

// fn main() {
//     use std::io;
//     use std::io::prelude::*;

//     print!("taskshell> ");
//     let _ = io::stdout().flush();
//     for line in io::stdin().lock().lines() {
//         let line = match line {
//             Ok(line) => line,
//             Err(e) => panic!("{}", e),
//         };

//         let args: Vec<&str> = line.split_ascii_whitespace().collect();
//         let command = match args.get(0) {
//             Some(c) => *c,
//             None => {
//                 println!("no non whitespace input given");
//                 continue;
//             }
//         };

//         match command {
//             "status" => status(args),
//             _ => println!("{} - command not implemented", command)
//         }
//         print!("taskshell> ");
//         let _ = io::stdout().flush();
//     }

//     let args = std::env::args();
//     if args.len() < 2 {
//         panic!("dont play with me");
//     }

//     let args = args.collect::<Vec<String>>();

//     let formatted_request = format!("{}\n", request);
//     let bytes = formatted_request.as_bytes();

//     let socket_path = "/tmp/.taskmaster.sock";

//     let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
//     let _ = write_request(&mut unix_stream, bytes);
//     let _ = read_from_stream(&mut unix_stream);
// }

use rustyline::DefaultEditor;
use rustyline::{config::Configurer, error::ReadlineError};

fn start(args: Vec<&str>) {
    let status_request_type = match args.len() {
        2 => tasklib::jsonrpc::JsonRPCRequestParams::Start(args.get(1).unwrap().to_string()),
        _ => {
            println!("wrong format - status command should be formated like - stop PROCESS_NAME");
            return;
        }
    };
    let request = serde_json::to_string(&JsonRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "start".to_owned(),
        params: status_request_type,
    });

    let request = match request {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let bytes = format!("{}\n", request);
    let bytes = bytes.as_bytes();

    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, bytes);
    let resp = read_from_stream(&mut unix_stream);

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    println!("{}", resp);
}
fn restart(args: Vec<&str>) {
    let status_request_type = match args.len() {
        2 => tasklib::jsonrpc::JsonRPCRequestParams::Restart(args.get(1).unwrap().to_string()),
        _ => {
            println!("wrong format - status command should be formated like - restart PROCESS_NAME");
            return;
        }
    };
    let request = serde_json::to_string(&JsonRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "restart".to_owned(),
        params: status_request_type,
    });

    let request = match request {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let bytes = format!("{}\n", request);
    let bytes = bytes.as_bytes();

    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, bytes);
    let resp = read_from_stream(&mut unix_stream);

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    println!("{}", resp);
}
fn stop(args: Vec<&str>) {
    let status_request_type = match args.len() {
        2 => tasklib::jsonrpc::JsonRPCRequestParams::Stop(args.get(1).unwrap().to_string()),
        _ => {
            println!("wrong format - status command should be formated like - stop PROCESS_NAME");
            return;
        }
    };
    let request = serde_json::to_string(&JsonRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "stop".to_owned(),
        params: status_request_type,
    });

    let request = match request {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let bytes = format!("{}\n", request);
    let bytes = bytes.as_bytes();

    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, bytes);
    let resp = read_from_stream(&mut unix_stream);

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    println!("{}", resp);
}

fn halt(args: Vec<&str>) {
    let status_request_type = match args.len() {
        1 => tasklib::jsonrpc::JsonRPCRequestParams::Halt,
        _ => {
            println!("wrong format - status command should be formated like - halt");
            return;
        }
    };
    let request = serde_json::to_string(&JsonRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "halt".to_owned(),
        params: status_request_type,
    });

    let request = match request {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let bytes = format!("{}\n", request);
    let bytes = bytes.as_bytes();

    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, bytes);
    let resp = read_from_stream(&mut unix_stream);

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    println!("{}", resp);
}

fn reload(args: Vec<&str>) {
    let status_request_type = match args.len() {
        1 => tasklib::jsonrpc::JsonRPCRequestParams::Reload,
        _ => {
            println!("wrong format - status command should be formated like - reload");
            return;
        }
    };
    let request = serde_json::to_string(&JsonRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "reload".to_owned(),
        params: status_request_type,
    });

    let request = match request {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let bytes = format!("{}\n", request);
    let bytes = bytes.as_bytes();

    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, bytes);
    let resp = read_from_stream(&mut unix_stream);

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    println!("{}", resp);
}

fn status(args: Vec<&str>) {
    let status_request_type = match args.len() {
        1 => tasklib::jsonrpc::JsonRPCRequestParams::Status,
        2 => tasklib::jsonrpc::JsonRPCRequestParams::StatusSingle(args.get(1).unwrap().to_string()),
        _ => {
            println!("wrong format - status command should be formated like - `status` or `status PROCESS_NAME`");
            return;
        }
    };
    let request = serde_json::to_string(&JsonRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: "status".to_owned(),
        params: status_request_type,
    });

    let request = match request {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    let bytes = format!("{}\n", request);
    let bytes = bytes.as_bytes();

    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, bytes);
    let resp = read_from_stream(&mut unix_stream);

    let resp = match resp {
        Ok(r) => r,
        Err(e) => {
            println!("{}", e);
            return;
        }
    };

    println!("{}", resp);
}

fn handle(line: String) {
    let args: Vec<&str> = line.split_ascii_whitespace().collect();
    let command = match args.get(0) {
        Some(c) => *c,
        None => {
            println!("no non whitespace input given");
            return;
        }
    };

    match command {
        "status" => status(args),
        "stop" => stop(args),
        "restart" => restart(args),
        "reload" => reload(args),
        "start" => start(args),
        "halt" => halt(args),
        _ => println!("{} - command not implemented", command),
    }
}

fn main() -> rustyline::Result<()> {
    // `()` can be used when no completer is required
    let mut rl = DefaultEditor::new()?;
    rl.set_auto_add_history(true);
    if rl.load_history("history.txt").is_err() {
        println!("No previous history.");
    }
    loop {
        let readline = rl.readline("taskshell> ");
        match readline {
            Ok(line) => {
                handle(line);
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
    rl.save_history("history.txt")
}
