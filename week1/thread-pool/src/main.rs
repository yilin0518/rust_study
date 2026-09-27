use std::io::{Read, Result, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::sync::{mpsc, Arc, Mutex};

fn handle_connect(mut stream: TcpStream, addr: SocketAddr) -> Result<()> {
    loop {
        let mut buf = [0; 1024];
        let n = stream.read(&mut buf)?;
        if n == 0 {
            return Ok(());
        }
        println!("Get {} from {addr}", String::from_utf8_lossy(&buf[..n]));
        stream.write_all(&buf[..n])?;
    }
}

type Job = (TcpStream, SocketAddr);

fn main() {
    let (sender, recevier) = mpsc::channel::<Job>();
    let receiver = Arc::new(Mutex::new(recevier));
    for worker_id in 0..2 {
        let receiver = Arc::clone(&receiver);
        thread::spawn(move || {
            loop {
                let job = {
                    let receiver = receiver.lock().unwrap();
                    receiver.recv()
                };
                if let Ok((stream, addr)) = job {
                    println!("Worker {worker_id} got a job from {addr}");
                    match handle_connect(stream, addr) {
                        Ok(_) => {
                            println!("{worker_id}: {addr} disconnected.");
                        }
                        Err(e) => {
                            println!("{worker_id}: {addr} happened some wrong: {e}");
                        }
                    }
                }
                else {
                    break;
                }
            }
        });
    }

    let lintener = TcpListener::bind("127.0.0.1:8080").unwrap();
    loop {
        let (stream, addr) = lintener.accept().unwrap();
        println!("Get a connection from {addr}");
        sender.send((stream, addr)).expect("All workers exist.");
    }
}
