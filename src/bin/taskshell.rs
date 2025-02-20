use std::{
    io::{Read, Write},
    os::unix::net::UnixStream,
};

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

    let input = &args.collect::<Vec<String>>()[1..].join(" ");
    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream, input.as_bytes());
    let _ = read_from_stream(&mut unix_stream);
}
