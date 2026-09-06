use std::io::{Read, Write};
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Server listening on 127.0.0.1:8080");
    while let Ok((mut stream, addr)) = listener.accept() {
        println!("client established: {addr}");
        let mut buffer = [0u8; 1024];
        while let Ok(n) = stream.read(&mut buffer) {
            if n == 0 {
                println!("client disconnected");
                break;
            }
            println!("Received {n} bytes: {:?}", &buffer[..n]);
            stream.write_all(&buffer[..n]).unwrap();
        }
    }
}
