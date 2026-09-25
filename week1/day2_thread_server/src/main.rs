use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8080").unwrap();
    println!("Server listening on 127.0.0.1:8080");
    let thread_cnts = Arc::new(Mutex::<usize>::new(0));
    while let Ok((stream, addr)) = listener.accept() {
        println!("client established: {addr}");
        let thread_cnts = Arc::clone(&thread_cnts);
        let _: std::thread::JoinHandle<()> = std::thread::spawn(move || {
            let mut cnts = thread_cnts.lock().unwrap();
            *cnts += 1;
            println!("Current thread count: {}, after {}", *cnts, addr);
            drop(cnts);
            match handle_connection(stream, addr) {
                Ok(_) => println!("Connection with {addr:?} closed successfully"),
                Err(e) => eprintln!("Error handling connection with {addr:?}: {e}"),
            }
            let mut cnts = thread_cnts.lock().unwrap();
            *cnts -= 1;
            println!("Current thread count: {}, after {} is disconnected", *cnts, addr);
        });
    }
}

fn handle_connection(mut stream: TcpStream, addr: SocketAddr) -> Result<(), std::io::Error> {
    let mut buffer = [0u8; 1024];
    loop {
        match stream.read(&mut buffer) {
            Ok(n) => {
                if n == 0 {
                    return Ok(());
                }
                println!(
                    "Received {n} bytes: {:?} from client {addr:?}",
                    String::from_utf8_lossy(&buffer[..n])
                );
                stream.write_all(&buffer[..n])?;
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}
