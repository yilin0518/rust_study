# Day 2：一连接一线程的 TCP 服务端

## 今天要解决的问题

Day 1 的服务端只有一个线程。它处理客户端 A 时可能停在：

```rust
stream.read(&mut buffer)
```

如果 A 保持连接却暂时不发送数据，线程就会阻塞，不能回到 `accept()` 处理客户端 B。

今天的目标是：**主线程持续接受连接，每个连接交给一个独立 OS 线程处理。**

```text
主线程
  accept A ──→ 线程 A：处理 A 的 read / echo 循环
  accept B ──→ 线程 B：处理 B 的 read / echo 循环
  accept C ──→ 线程 C：处理 C 的 read / echo 循环
```

因此，客户端 A 的阻塞读取只会阻塞线程 A，不会阻止主线程接受 B，也不会阻止线程 B 处理 B。

## 第一步：提取连接处理函数

Day 1 的“读取、遇到零字节断开、回写”逻辑属于一条连接。先将它提取成类似的函数：

```rust
fn handle_connection(stream: /* 需要哪个类型？ */, addr: /* 需要哪个类型？ */) {
    // 复用 Day 1 的 read / echo 循环
}
```

思考：函数需要拥有 `stream`，还是只借用 `&mut stream`？当该函数将在新线程中运行时，这个选择为什么重要？

## `thread::spawn` 与 `move`

`thread::spawn` 接收一个闭包，并在新 OS 线程中运行它。

```rust
std::thread::spawn(|| {
    // 新线程执行这里
});
```

如果闭包只是借用主线程局部变量 `stream`，Rust 通常会拒绝：主线程的当前作用域可能先结束，而新线程仍在访问这个借用。那会产生悬垂引用风险。

因此，连接处理通常需要这样的所有权方向：

```text
accept() 创建并返回一个 TcpStream 值
        ↓
主线程将该值 move 到新线程的闭包
        ↓
新线程独占该 TcpStream，并运行连接处理循环
```

`move` 表示捕获的值所有权转移进闭包。它不意味着必然复制数据；对 `TcpStream` 而言，通常是 socket 句柄的所有权移动。移动之后，主线程不能继续使用原来的 `stream` 变量。

## `Send` 与 `'static`

在线程间转移一个类型时，Rust 要求它实现 `Send`。`Send` 表示该值的所有权可以安全地从一个线程转移到另一个线程。

`thread::spawn` 的闭包还需要能独立存活：新线程何时结束无法由当前函数保证。因此闭包不能引用当前栈帧中可能消失的数据，这常表现为 `'static` 相关的编译错误。将 `TcpStream` 直接 `move` 进去能满足这个生命周期方向，因为它是闭包拥有的值。

## 预期代码形状

这不是完整答案，而是组织代码时应达到的层次：

```text
listener 的 accept 循环
    对每个 Ok((stream, addr))：
        启动线程
            该线程拥有 stream
            调用 handle_connection(stream, addr)
```

主线程不再直接调用连接的 `read()`；它负责尽快回到下一次 `accept()`。

## Windows 并发验证

启动服务端：

```powershell
cargo run -p day2_thread_server
```

在两个不同的 PowerShell 窗口中各自建立 `TcpClient`，发送不同文本并读取回显。关键实验是：保持客户端 A 的连接不关闭，再让 B 发送数据。

预期：B 仍能迅速收到自己的回显。因为 B 由独立线程处理，而不是等待 A 的 `read()` 返回。

## 这个模型的代价

它解决了“一个慢客户端阻塞所有客户端”，却引入了新的问题：每条连接都占用 OS 线程。大量线程会带来栈内存、调度、上下文切换和系统资源开销。这个限制是 Day 3 学习 Future 和 Day 4 使用 Tokio 的动机。

## 自测问题

1. `move` 为什么能解决跨线程闭包对 `stream` 的所有权问题？
2. `move` 后，主线程是否还能读取同一个 `stream`？
3. `Send` 在这里保证什么？它是否表示值可被多个线程同时共享？
4. 一连接一线程为什么能处理两个慢客户端？
5. 为什么它不适合无上限地创建线程？

## 推荐资料

### 必读：中文

1. [《Rust 程序设计语言》：使用线程同时运行代码](https://rustwiki.org/zh-CN/book/ch16-01-threads.html)

   重点阅读 `thread::spawn`、`JoinHandle` 和 `move` 闭包。阅读时思考：示例中的值为什么需要由新线程拥有？

2. [《Rust 程序设计语言》：使用 `Sync` 和 `Send` trait 的可扩展并发](https://rustwiki.org/zh-CN/book/ch16-04-extensible-concurrency-sync-and-send.html)

   重点区分：`Send` 表示所有权可跨线程转移；`Sync` 表示共享引用可跨线程使用。本单元首先需要理解 `Send`。

### 必读：英文官方文档

3. [`std::thread` module](https://doc.rust-lang.org/std/thread/)

   只需阅读 “The threading model” 和 “Spawning a thread”。注意 OS 线程拥有独立栈，以及 `spawn` 返回 `JoinHandle`。

### 选读

4. [The Rustonomicon: Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)

   用于加深类型层面的理解。当前不需要学习手动实现 `Send` 或 `Sync`，也不要因此提前进入 unsafe Rust。
