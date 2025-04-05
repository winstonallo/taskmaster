use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

use serde_json::json;
use tasklib::jsonrpc::JsonRPCRequest;

fn read_from_stream(unix_stream: &mut UnixStream) -> Result<(), String> {
    let mut buf = String::new();

    unix_stream.read_to_string(&mut buf).map_err(|e| format!("{}", e))?;

    println!("{}", buf);

    Ok(())
}

fn write_request(unix_stream: &mut UnixStream, request: &[u8]) -> Result<(), String> {
    unix_stream.write(request).map_err(|e| format!("{}", e))?;
    unix_stream.shutdown(std::net::Shutdown::Write).map_err(|e| format!("{}", e))?;

    Ok(())
}

fn main() {
    let args = std::env::args();
    if args.len() < 2 {
        panic!("dont play with me");
    }

    let args = args.collect::<Vec<String>>();

    let request = serde_json::to_string(&JsonRPCRequest {
        jsonrpc: "2.0".to_string(),
        id: 1,
        method: args[1].clone(),
        params: Some(json!({
            "name": "ls_0",
        })),
    })
    .expect("serde failed");

    let formatted_request = format!("{}\n", request);
    let bytes = formatted_request.as_bytes();

    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, bytes);
    let _ = read_from_stream(&mut unix_stream);
}
