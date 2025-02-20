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

fn write_request(unix_stream: &mut UnixStream) -> Result<(), String> {
    unix_stream.write(b"start").map_err(|e| format!("{}", e))?;
    unix_stream.shutdown(std::net::Shutdown::Write).map_err(|e| format!("{}", e))?;

    Ok(())
}

fn main() {
    let socket_path = "/tmp/.taskmaster.sock";

    let mut unix_stream = UnixStream::connect(socket_path).expect("could not create stream");
    let _ = write_request(&mut unix_stream);
    let _ = read_from_stream(&mut unix_stream);
}
