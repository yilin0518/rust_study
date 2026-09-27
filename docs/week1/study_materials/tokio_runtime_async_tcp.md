# 学习主题：Tokio Runtime、异步 TCP 与任务调度

> 本主题承接 `Future → poll → Pending → Waker → 再次 poll`。
> 目标不是记住 Tokio API，而是把刚刚手写的执行模型连接到真实 TCP I/O。
> 按“读代码 → 预测 → 实现 → 运行 → 对照 Day 2”的顺序完成，可以分多次学习。

## 1. 为什么现在引入 Tokio

Day 2 的服务器用一个 OS 线程处理一个连接。连接没有数据时，对应线程阻塞在
`std::net::TcpStream::read`。这种模型简单直接，但大量空闲连接也会占有大量线程。

上一主题已经建立了下面的模型：

```text
Future 被执行器 poll
        ↓
暂时不能完成 → Pending，并安排 wake
        ↓
执行器处理其他任务
        ↓
事件发生 → wake → 再次 poll
        ↓
Ready
```

真实异步网络还缺少两个重要部分：

1. 能与操作系统的非阻塞 socket 配合的异步 I/O 类型。
2. 能调度任务、等待 I/O 事件并调用 Waker 的运行时。

Tokio 提供这些基础设施。它不会改变 TCP 的语义，也不会让阻塞代码自动变成异步代码。

### 完成后应能解释

1. Tokio runtime 除了轮询 Future，还承担哪些职责。
2. `#[tokio::main]` 为什么能让最外层 Future 开始运行。
3. `.await` 在 Future `Ready` 与 `Pending` 时分别发生什么。
4. `tokio::spawn` 创建的是任务还是 OS 线程。
5. 为什么一个任务等待 socket 时，runtime 线程还能运行其他任务。
6. `async move`、`Send` 与 `'static` 为什么会出现在 `tokio::spawn` 附近。
7. 哪些 TCP 规则不会因为使用 Tokio 而改变。
8. 为什么在异步任务里调用阻塞式 `read`、`sleep` 或长时间 CPU 循环仍有问题。

## 2. 前置知识、范围与产物

### 已完成的前置知识

- 阻塞 TCP Echo Server。
- 一连接一线程，以及 `move`、`Send`、`Arc<Mutex<_>>`。
- Future 是惰性的值，执行器通过 `poll` 推进它。
- `Pending`、Waker 与再次轮询的关系。
- TCP 是字节流，`read() == 0` 表示对端有序关闭写方向。

### 本主题的产物

创建一个按内容命名的新 crate：

```text
week1/tokio_echo_lab
```

最终得到一个能并发处理多个连接的最小 Tokio Echo Server，并写出它与 Day 2
线程版的直接对照。

### 本主题暂不处理

- 空闲超时、取消和优雅关闭。
- 任务集合管理与完整的错误恢复策略。
- TCP 消息分帧。
- `select!`、异步 channel 和异步锁。
- 手写 reactor、操作系统事件循环或 Tokio 内部实现。
- 性能基准与生产部署。

这些内容将在后续主题中建立在当前服务器上继续演进。

## 3. 核心知识模型

### 3.1 Runtime 不只是一个 `loop { poll() }`

从本主题需要的抽象层次看，Tokio runtime 包含：

```text
Runtime
├── scheduler / executor：保存并调度可运行任务
├── I/O driver：监听 socket 就绪事件并触发 Waker
└── timer driver：启用相应 feature 时支持异步定时器（后续主题使用）
```

你手写的最小执行器使用 `mpsc` 接收 wake 通知。Tokio 则把任务调度和真实 I/O
事件连接起来。当 `accept().await` 或 `read().await` 暂时不能完成时，任务可以返回
`Pending`，而不是阻塞 runtime 线程。

### 3.2 `#[tokio::main]` 建立最外层驱动

普通 Rust 程序的入口必须是同步 `fn main()`。下面的属性宏：

```rust
#[tokio::main]
async fn main() {
    // ...
}
```

概念上接近：

```rust,ignore
fn main() {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        // 原 async main 的函数体
    });
}
```

这不是精确的宏展开，但足以说明两件事：

1. `async fn main()` 仍会产生 Future。
2. runtime 的 `block_on` 负责驱动最外层 Future，直到它完成。

本主题先使用：

```rust
#[tokio::main(flavor = "current_thread")]
```

单线程 runtime 能更清楚地证明“并发不等于并行”：多个连接任务即使共享一个
runtime 线程，也能在 I/O 等待点交替推进。完成核心实验后，再去掉 `flavor`，观察
默认多线程 runtime；不要依赖具体线程或日志顺序。

### 3.3 `.await` 是轮询边界，不保证每次都暂停

表达式：

```rust
let n = stream.read(&mut buffer).await?;
```

可以从概念上理解为：

```text
轮询 read Future
├── Ready(Ok(n)) → 立刻得到 n，继续执行
├── Ready(Err(e)) → 传播错误
└── Pending       → 保存当前任务状态，把控制权交回 runtime
```

`.await` 不等于创建线程，也不等于固定地暂停一次。如果数据已经就绪，Future 可能
第一次被 poll 就返回 `Ready`，代码会继续执行。

### 3.4 Tokio I/O 与标准库阻塞 I/O

| Day 2 | Tokio 版本 | 等待时的主要行为 |
| --- | --- | --- |
| `std::net::TcpListener` | `tokio::net::TcpListener` | Tokio 类型与 I/O driver 配合 |
| `listener.accept()` | `listener.accept().await` | 无连接时挂起任务 |
| `std::io::Read` | `tokio::io::AsyncReadExt` | 提供返回 Future 的读取方法 |
| `stream.read(...)` | `stream.read(...).await` | 无数据时挂起任务 |
| `std::io::Write` | `tokio::io::AsyncWriteExt` | 提供异步写入方法 |
| `stream.write_all(...)` | `stream.write_all(...).await` | 写入暂不能继续时挂起任务 |

异步版本的关键不是多了 `.await` 字样，而是底层 socket 使用非阻塞模式并与 runtime
的 I/O driver 登记就绪兴趣。

### 3.5 Task 与 OS 线程不是一回事

Day 2：

```rust
std::thread::spawn(move || {
    handle_connection(stream, addr)
});
```

Tokio：

```rust,ignore
tokio::spawn(async move {
    handle_connection(stream, addr).await
});
```

对照：

| OS 线程 | Tokio task |
| --- | --- |
| 由操作系统调度 | 由 Tokio scheduler 调度 |
| 有独立线程栈 | Future 状态只保存跨暂停点需要的数据 |
| 阻塞调用主要阻塞该线程 | 阻塞调用会阻塞承载该任务的 runtime 线程 |
| 数量通常相对昂贵 | 任务通常轻量，可远多于 runtime 线程 |
| `thread::spawn` 返回线程 JoinHandle | `tokio::spawn` 返回任务 JoinHandle |

`tokio::spawn` 只是把 Future 注册为可独立调度的任务，不保证创建一个新的 OS 线程。

### 3.6 `async move`、`Send` 与 `'static`

连接由 accept 循环创建，但连接处理任务可能在 accept 循环进入下一次迭代后继续存在。
因此任务应拥有自己的 `TcpStream` 和地址：

```rust,ignore
tokio::spawn(async move {
    handle_connection(stream, addr).await
});
```

- `move`：把捕获的值移入 async block。
- `'static` 约束：任务不能保存可能先失效的外部借用；不表示任务一定存活到程序结束。
- `Send`：默认多线程 scheduler 可能在任务暂停后把它放到另一个 runtime 线程继续执行，
  因此跨 `.await` 保存的状态需要满足相应的 `Send` 要求。

这与 Day 2 的线程闭包有相似之处，但 task 保存的是可暂停 Future 的状态。

### 3.7 Tokio 不改变 TCP 语义

下面这些规则全部保留：

- TCP 仍是有序、可靠的字节流，不是消息协议。
- 一次客户端写入不保证对应服务端的一次读取。
- 一次读取可能只拿到部分数据，也可能拿到多次写入合并后的数据。
- `read().await` 返回 `Ok(0)` 仍表示 EOF，需要退出连接循环。
- 必须只处理 `&buffer[..n]`。
- `write_all().await` 解决“把给定缓冲区全部交给写入过程”，不提供消息边界。

Tokio 改变的是等待方式和调度方式，不是传输协议。

### 3.8 协作式调度要求任务主动让出执行权

下面的代码即使放进 `async fn` 也不会自动变得友好：

```rust,ignore
async fn bad_task() {
    loop {
        // 没有 await、没有返回 Pending、没有结束
    }
}
```

它可能长期占用一个 runtime worker。类似问题还包括：

```rust,ignore
std::thread::sleep(...);      // 阻塞当前 runtime 线程
std::net::TcpStream::read(...); // 可能阻塞当前 runtime 线程
大量不让出的 CPU 计算;
```

本主题的网络路径只使用 Tokio I/O。确实无法避免的阻塞操作以后使用专门机制隔离，
不要在这里提前混用。

## 4. 实验路线

### 阶段 A：观察 runtime、task 与 `.await`

#### A1. 创建 crate 和最小依赖

在工作区根目录运行：

```powershell
cargo new week1/tokio_echo_lab
cargo add tokio --package tokio_echo_lab --features rt-multi-thread,macros,net,io-util
```

如果使用 `Cargo.toml` 手工添加，等价依赖形状为：

```toml
[dependencies]
tokio = { version = "1", features = ["rt-multi-thread", "macros", "net", "io-util"] }
```

每个 feature 的原因：

- `rt-multi-thread`：运行时与默认多线程 scheduler。
- `macros`：使用 `#[tokio::main]`。
- `net`：使用 Tokio TCP 类型。
- `io-util`：使用 `AsyncReadExt`、`AsyncWriteExt`。

不要先使用额外的网络框架；本主题需要直接看见 Tokio 与 Future 的连接。

#### A2. 运行一段完整的任务实验

暂时将 `src/main.rs` 写成：

```rust
async fn child(name: &'static str) {
    println!("{name}: before yield");
    tokio::task::yield_now().await;
    println!("{name}: after yield");
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    println!("main: start");

    let task_a = tokio::spawn(child("A"));
    let task_b = tokio::spawn(child("B"));

    println!("main: tasks spawned");

    task_a.await.unwrap();
    task_b.await.unwrap();

    println!("main: end");
}
```

运行前预测：

1. `tokio::spawn` 返回后，两个任务是否一定已经完成？
2. `yield_now().await` 前后的两行是否一定相邻？
3. `task_a.await` 是阻塞 OS 线程，还是挂起 `main` 任务？
4. 单线程 runtime 能否让 A、B 都取得进展？

运行两次，记录实际顺序。不要把一次输出顺序当成 Tokio 的稳定保证。

完成观察后，用自己的话把它映射到上一主题：

```text
spawn → 任务进入调度系统
yield_now().await → 当前任务让出执行权
JoinHandle.await → 等待目标任务完成
runtime → 选择下一个可运行任务
```

### 阶段 B：把 Day 2 的阻塞 API 翻译成异步 API

先不要添加 `tokio::spawn`。目标是得到一个“使用异步 I/O、但仍按连接顺序处理”的版本，
从而证明：

> 使用 `async fn` 不等于自动获得多连接并发。

#### B1. Listener 骨架

```rust,ignore
use std::io;
use tokio::net::TcpListener;

#[tokio::main(flavor = "current_thread")]
async fn main() -> io::Result<()> {
    // TODO：异步绑定 127.0.0.1:8080
    // TODO：打印监听地址

    loop {
        // TODO：等待并接收一个连接
        // TODO：直接 await handle_connection，暂时不要 spawn
    }
}
```

需要查阅的签名：

```rust,ignore
TcpListener::bind(...).await
listener.accept().await
```

预测：当 A 已连接但不发送数据，主任务在连接处理函数的 `read().await` 处等待时，B 的
TCP 连接可能被操作系统接受进队列，但你的 Rust accept 循环是否会立即处理 B？

#### B2. 连接处理骨架

```rust,ignore
use std::io;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

async fn handle_connection(
    mut stream: TcpStream,
    addr: SocketAddr,
) -> io::Result<()> {
    let mut buffer = [0u8; 1024];

    loop {
        // TODO：异步读取
        // TODO：n == 0 时结束连接
        // TODO：只打印并回写 buffer[..n]
        // TODO：异步 write_all
    }
}
```

迁移时逐项对照 Day 2，不要复制后机械地给每行加 `.await`：

```text
std::io::{Read, Write}
    ↓
tokio::io::{AsyncReadExt, AsyncWriteExt}

std::net::TcpStream
    ↓
tokio::net::TcpStream
```

运行后只连接一个客户端，验证 Echo 和 `read().await == 0` 的断开行为。

### 阶段 C：用 task 获得多连接并发

阶段 B 中，主任务直接执行：

```rust,ignore
handle_connection(stream, addr).await?;
```

这意味着 accept 循环要等当前连接处理完成才能进入下一轮。现在把每个连接处理 Future
提交成独立任务：

```rust,ignore
loop {
    let (stream, addr) = listener.accept().await?;

    tokio::spawn(async move {
        // TODO：await handle_connection
        // TODO：连接级错误只记录，不让整个监听器退出
    });
}
```

实现前说明以下三个所有权问题：

1. `stream` 原本由谁拥有？
2. 为什么 async block 需要 `move`？
3. accept 循环进入下一次迭代后，旧任务为什么仍能安全使用自己的 `stream`？

#### 两客户端验证

保持服务器运行。打开 PowerShell 客户端 A，只连接但不发送：

```powershell
$clientA = [System.Net.Sockets.TcpClient]::new("127.0.0.1", 8080)
$streamA = $clientA.GetStream()
```

再打开客户端 B，连接、发送并读取 Echo：

```powershell
$clientB = [System.Net.Sockets.TcpClient]::new("127.0.0.1", 8080)
$streamB = $clientB.GetStream()
$dataB = [Text.Encoding]::UTF8.GetBytes("from B")
$streamB.Write($dataB, 0, $dataB.Length)

$bufferB = New-Object byte[] 1024
$nB = $streamB.Read($bufferB, 0, $bufferB.Length)
[Text.Encoding]::UTF8.GetString($bufferB, 0, $nB)
```

关键验证：不关闭 A，B 也应及时收到 `from B`。

最后清理客户端：

```powershell
$clientB.Close()
$clientA.Close()
```

解释观察：

```text
任务 A 在 read().await 处 Pending
        ↓
runtime 线程没有被阻塞
        ↓
主任务继续 accept B
        ↓
任务 B 读取并回写数据
```

### 阶段 D：证明“并发不要求一个任务一个线程”

保留：

```rust
#[tokio::main(flavor = "current_thread")]
```

在接受连接和处理数据处临时打印：

```rust
println!("thread = {:?}", std::thread::current().id());
```

重复 A 空闲、B Echo 的实验。如果日志显示同一个线程 ID，不要得出“任务永远固定在线程”
的结论；当前实验只证明：

> 单个 runtime 线程也能并发推进多个 I/O 任务，因为等待中的任务会让出执行权。

之后改回：

```rust
#[tokio::main]
```

再次运行。默认多线程 runtime 可能让任务在不同 worker 上运行，也可能因为工作量小而主要
出现在少数线程上；不要依赖具体分布。

### 阶段 E：故意加入阻塞，观察协作式调度的边界（可选）

只在 `current_thread` 实验版本中临时加入：

```rust,ignore
std::thread::sleep(std::time::Duration::from_secs(5));
```

把它放在某个连接任务收到数据之后，再让另一个客户端发送数据。预测并观察第二个任务是否
能及时取得进展。

实验后必须删除这行。结论应是：

```text
阻塞调用占住 runtime 线程
        ↓
同一线程上的其他任务也无法被 poll
```

异步任务轻量不代表其中可以随意执行阻塞工作。

## 5. 运行、检查与实验记录

从工作区根目录运行：

```powershell
cargo check -p tokio_echo_lab
cargo run -p tokio_echo_lab
cargo fmt -p tokio_echo_lab -- --check
cargo clippy -p tokio_echo_lab -- -D warnings
```

如果包名使用连字符，以实际 `Cargo.toml` 中的 `package.name` 为准。

每个阶段记录：

```text
预测：
实际输出或网络现象：
代码停在哪个 .await：
此时哪个任务是 Pending：
什么事件会唤醒它：
与 Day 2 的差异：
```

建议至少保存以下证据：

1. 单客户端 Echo。
2. A 空闲时 B 仍能收到 Echo。
3. `current_thread` 模式下两个连接任务仍能并发推进。
4. 客户端关闭后，对应任务观察到 `read().await == 0` 并退出。

## 6. 常见错误与分级提示

| 现象 | 先检查什么 |
| --- | --- |
| 找不到 `tokio::main` | 是否启用了 `macros` 和 runtime feature |
| 找不到 `tokio::net` | 是否启用了 `net` feature |
| 没有 `read` / `write_all` 方法 | 是否导入 `AsyncReadExt` / `AsyncWriteExt`，并启用 `io-util` |
| `.await` 只能用于 async 上下文 | 当前函数是否为 `async fn` 或 async block |
| B 已连接但收不到 Echo | 是否仍在主任务中直接 await A 的 handler，而没有 spawn |
| spawn 后提示借用可能越界 | async block 是否需要取得值所有权，考虑 `async move` |
| Future 不是 `Send` | 是否有非 `Send` 值或锁 guard 跨越 `.await` |
| CPU 很高且其他任务卡住 | 是否存在没有 `.await` 的长循环或持续计算 |
| 所有任务同时停顿 | 是否在 async task 中调用阻塞式 sleep、I/O 或长时间持锁 |
| 客户端断开后任务不退出 | 是否处理了 `read().await? == 0` |
| Echo 带有多余零字节 | 是否错误回写整个 buffer，而不是 `buffer[..n]` |

求助时按以下层级增加提示：

1. **模型提示：** 指出问题属于 runtime、I/O、task、所有权还是 TCP 语义。
2. **API 提示：** 查看相关 Tokio 类型、extension trait 和方法签名。
3. **结构提示：** 只画出 accept 主任务与 connection task 的关系。
4. **局部代码：** 展示产生错误或不能推进的几行，并解释编译器或运行时现象。
5. **完整实现：** 仅在自己已经完成严肃尝试后使用，并逐行对照 Day 2。

## 7. Day 2 与 Tokio 版本的最终对照

完成后自行填写“实际代码位置”一列：

| 问题 | Day 2 线程版 | Tokio 版 | 实际代码位置 |
| --- | --- | --- | --- |
| 调度单位 | OS 线程 | Tokio task | |
| 连接等待 | 主线程阻塞在 `accept` | 主任务在 `accept().await` 处挂起 | |
| 数据等待 | 连接线程阻塞在 `read` | 连接任务在 `read().await` 处挂起 | |
| 并发方式 | 每连接 `thread::spawn` | 每连接 `tokio::spawn` | |
| 所有权转移 | `move` 线程闭包 | `async move` block | |
| 等待恢复来源 | OS 唤醒阻塞线程 | I/O driver 调用 Waker，scheduler 重调度任务 | |
| EOF | `read() == 0` | `read().await == 0` | |
| TCP 字节流 | 是 | 仍然是 | |

## 8. 完成证据与复盘题

### 完成证据

- 自己实现并运行 `week1/tokio_echo_lab`，而不是只阅读最终代码。
- 能在代码中指出 `accept().await` 和 `read().await` 各自可能返回 `Pending` 的位置。
- A 保持空闲时，B 不必等待 A 关闭就能收到 Echo。
- 在 `current_thread` runtime 下验证多个任务也能并发处理连接。
- 正确处理 `n == 0`、有效字节切片以及连接级错误。
- 能将一次网络等待完整解释为 `Pending → I/O 就绪 → wake → 再次 poll`。
- 能说明 task 与线程、并发与并行、异步与非阻塞之间的区别。
- `cargo check`、`cargo fmt --check` 和 `cargo clippy` 通过。

### 复盘题

1. `#[tokio::main]` 解决了最外层 Future 的什么问题？
2. `.await` 是否必然让任务暂停？为什么？
3. `accept().await` 没有连接时，谁保存等待状态，谁在连接到来时唤醒任务？
4. 为什么 `tokio::spawn` 不等于 `thread::spawn`？
5. 单线程 runtime 为什么也能同时服务两个空闲或活跃连接？
6. `async move` 将哪些值移动到了任务中？这与 Day 2 有何相似之处？
7. 为什么把 `std::net::TcpStream::read` 放进 `async fn` 仍可能阻塞 runtime？
8. `read().await == 0` 表示什么？为什么必须退出循环？
9. Tokio 是否解决了 TCP 消息边界问题？
10. 如果某个任务在两次 `.await` 之间执行很久，其他任务可能受到什么影响？

## 9. 参考资料

### 必读，按阶段阅读

1. [Tokio 官方教程概览](https://tokio.rs/tokio/tutorial)：阶段 A 前阅读 runtime 的三个主要组成部分。
2. [Tokio `#[main]` 官方文档](https://docs.rs/tokio/latest/tokio/attr.main.html)：阶段 A 阅读 runtime flavor 与概念展开。
3. [Tokio 官方教程：Spawning](https://tokio.rs/tokio/tutorial/spawning)：阶段 C 阅读 task、`async move`、`Send` 与 `'static`。
4. [Tokio `TcpListener` 官方文档](https://docs.rs/tokio/latest/tokio/net/struct.TcpListener.html)：阶段 B 阅读 `bind`、`accept` 和 `poll_accept`。
5. [Tokio `AsyncReadExt` 官方文档](https://docs.rs/tokio/latest/tokio/io/trait.AsyncReadExt.html)：阶段 B 阅读 `read`。
6. [Tokio `AsyncWriteExt` 官方文档](https://docs.rs/tokio/latest/tokio/io/trait.AsyncWriteExt.html)：阶段 B 阅读 `write_all`。

### 选读

7. [Tokio 官方教程：I/O](https://tokio.rs/tokio/tutorial/io)：完成最小 Echo 后，对照官方 Echo 与 split 方式；本主题不要求使用 `io::copy`。
8. [Tokio 官方教程：Async in depth](https://tokio.rs/tokio/tutorial/async)：完成全部实验后，将 Tokio task 队列与上一主题的 channel 执行器对应起来。

阅读官方示例时不要直接复制完整服务器。先判断示例使用的 feature、错误处理和并发模型是否符合当前阶段。

## 10. 下一主题

本主题完成后，在同一个 Tokio 服务器上继续学习连接生命周期：

```text
多连接 Echo
    ↓
连接级错误边界
    ↓
空闲超时与取消
    ↓
停止 accept
    ↓
优雅关闭和任务收尾
```

之后再进入 TCP 分帧。不要因为 API 已经异步化，就忘记 TCP 仍然只提供字节流。
