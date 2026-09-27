use std::future::Future;
use std::task::{Context, Poll, Waker, Wake};
use std::pin::Pin;
use std::sync::{
    Arc,
    mpsc::{self, Sender}
};

async fn produce_number() -> i32 {
    println!("async body started");
    // TODO：打印 "async body started"
    42
    // TODO：返回一个整数
}

struct CountDown {
    remaining: i32,
}

impl Future for CountDown {
    type Output = &'static str;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();
        println!("poll called, remaining = {}", this.remaining);
        if this.remaining == 0 {
            Poll::Ready("done")
        } else {
            this.remaining -= 1;
            println!("Future requests another poll");
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

struct ChannelWake {
    sender: Sender<()>,
}

impl Wake for ChannelWake {
    fn wake(self: Arc<Self>) {
        self.sender.send(()).unwrap();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.sender.send(()).unwrap();
    }
}

fn main() {
    let mut future = Box::pin(CountDown{ remaining: 3 });

    let (sender, receiver) = mpsc::channel();
    let waker = Waker::from(Arc::new(ChannelWake {sender}));
    let mut context = Context::from_waker(&waker);

    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(value) => {
                println!("Future completed with value: {}", value);
                break;
            }
            Poll::Pending => {
                println!("Executor waiting for wake notification");
                receiver.recv().unwrap();
                println!("Executor received wake notification");
            }
        }
    }
}