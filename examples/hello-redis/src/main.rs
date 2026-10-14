use bytes::Bytes;
use mini_redis::{Result, client};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
enum Command {
    Get {
        key: String,
        response: Responder<Option<Bytes>>,
    },
    Set {
        key: String,
        value: Bytes,
        response: Responder<()>,
    },
}

type Responder<T> = oneshot::Sender<Result<T>>;

#[tokio::main]
async fn main() -> Result<()> {
    let mut client = client::connect("127.0.0.1:6379").await?;

    let (tx, mut rx) = mpsc::channel(32);
    let tx2 = tx.clone();

    let t1 = tokio::spawn(async move {
        let (response_tx, response_rx) = oneshot::channel();
        let cmd = Command::Set {
            key: "hello".to_string(),
            value: "world".into(),
            response: response_tx,
        };
        tx.send(cmd).await.unwrap();

        let response = response_rx.await;
        println!("SET response: {:?}", response);
    });

    let t2 = tokio::spawn(async move {
        let (response_tx, response_rx) = oneshot::channel();
        let cmd = Command::Get {
            key: "hello".to_string(),
            response: response_tx,
        };
        tx2.send(cmd).await.unwrap();
        let response = response_rx.await;
        println!("GET response: {:?}", response);
    });

    let manager = tokio::spawn(async move {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Command::Get { key, response } => {
                    let res = client.get(&key).await;
                    response.send(res).unwrap();
                }
                Command::Set { key, value, response } => {
                    let res = client.set(&key, value).await;
                    response.send(res).unwrap();
                }
            }
        }
    });

    t1.await.unwrap();
    t2.await.unwrap();
    manager.await.unwrap();
    Ok(())
}