use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, AsyncReadExt};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let mut socket = TcpStream::connect("127.0.0.1:8080").await?;

    socket.write_all(b"Hello world!").await?;

    let mut buf = vec![0; 1024];
    let n = socket.read(&mut buf).await?;
    println!("Received: {}", String::from_utf8_lossy(&buf[..n]));

    Ok(())
}
