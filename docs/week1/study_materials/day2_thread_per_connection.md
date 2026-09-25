# Day 2：一连接一线程的 TCP 服务端

## 1. 本单元位置

```text
Day 1：单线程阻塞 Echo
        ↓
Day 2：一连接一线程（本单元）
        ↓
Day 3：Future / Poll / Waker
        ↓
Day 4：Tokio 异步 Echo
```

Day 1 已经观察到：主线程处理客户端 A 时会阻塞在 A 的 `read()`，因而无法回到 `accept()` 处理客户端 B。

Day 2 不会让 socket 读取变成非阻塞。它通过增加 OS 线程，使每条连接的阻塞只影响自己的线程：

```text
main thread
  ├── accept A -> connection thread A -> read A
  ├── accept B -> connection thread B -> read B
  └── accept C -> connection thread C -> read C
```

## 2. 完整学习目标

完成 Day 2 后，你应该能够：

1. 将监听职责与单连接处理职责分离。
2. 解释 `TcpStream` 从 `accept()` 到连接线程的所有权路径。
3. 使用 `std::thread::spawn` 创建一连接一线程服务。
4. 解释闭包捕获、`move`、`Send` 和 `'static`。
5. 区分所有权移动、可变绑定、共享访问和深拷贝。
6. 使用两个保持连接的客户端验证真正的并发行为。
7. 理解 `JoinHandle`，以及为什么不能在 accept 循环中立即 `join()`。
8. 使用 `Arc<Mutex<T>>` 安全共享一个简单状态。
9. 解释为什么每条连接本身不需要 `Arc<Mutex<TcpStream>>`。
10. 说明一连接一线程的资源成本和扩展性边界。

## 3. 前置知识自检

开始前，不看笔记回答：

1. `accept()` 和 `read()` 分别在什么情况下阻塞？
2. `read()` 返回 `Ok(0)` 表示什么？
3. 为什么有效数据是 `&buffer[..n]`？
4. TCP 为什么不是消息协议？
5. 客户端 B 能完成 TCP 握手，是否表示 Rust 程序已经调用 `accept()` 取得它？
6. Day 1 中究竟是哪一个调用阻止主线程处理 B？

## 4. 教材正文：从阻塞服务到一连接一线程

### 4.1 并发、并行与阻塞不是同一个概念

学习线程前，需要区分三个经常被混用的词。

**阻塞（blocking）**描述一个执行线程的状态。当线程调用阻塞式 `accept()` 或 `read()`，而操作暂时无法完成时，操作系统会让该线程等待。等待期间，它不能继续执行该调用之后的 Rust 代码。

**并发（concurrency）**描述多个任务在同一时间段内都能取得进展。即使机器只有一个 CPU 核心，操作系统也可以在多个线程之间切换，使多个连接交替推进。

**并行（parallelism）**表示多个任务在同一时刻真正运行，通常需要多个 CPU 核心。

一连接一线程首先解决的是并发：客户端 A 等待网络输入时，客户端 B 的线程仍然可以运行。它不保证两个线程一定在不同核心上同时运行。

思考：如果机器只有一个 CPU 核心，一连接一线程是否仍能改善两个网络客户端的响应？为什么？

### 4.2 Day 1 为什么只能顺序处理

Day 1 只有一个 Rust 主线程：

```text
accept A
  -> read A
      -> A 暂时不发送数据
      -> 主线程在内核中等待
      -> 无法执行下一次 accept
```

注意，“程序没有再次调用 `accept()`”不一定表示客户端 B 无法完成 TCP 握手。监听 socket 在操作系统中通常有等待被应用取走的已建立连接队列。B 可能已经与内核完成连接建立，并且其数据可能暂存在内核接收缓冲区；但 Rust 程序尚未取得代表 B 的 `TcpStream`，因此不会执行 B 的业务逻辑。

可以把三个时刻分开：

```text
B 与服务器内核完成 TCP 握手
        ↓
B 进入 listener 的等待队列
        ↓
Rust 调用 accept，获得 B 的 TcpStream
        ↓
Rust 调用 read，读取 B 的字节
```

这解释了为什么 B 的 `TcpClient` 构造函数可能成功，但 B 仍迟迟收不到 Echo。

### 4.3 一连接一线程改变了什么

新架构把职责拆成：

```text
主线程：
  bind -> accept -> spawn -> 立即回到 accept

连接线程：
  拥有一条 TcpStream -> read/echo loop -> 断开 -> 退出
```

关键不变量是：

> 一条连接在当前设计中只由一个连接线程拥有和处理。

因此不需要为了“线程安全”把每条 `TcpStream` 包进 `Arc<Mutex<_>>`。如果只有一个线程访问一个值，独占所有权本身就是最简单的安全策略。

线程也没有改变 TCP 语义：TCP 仍是字节流，一次 `write` 仍不对应一次 `read`，`read() == 0` 仍表示有序 EOF。

### 4.4 函数边界也是所有权边界

Day 1 中连接处理代码嵌在 `main`，导致 listener 生命周期和连接生命周期混合。提取 handler 后：

```rust
fn handle_connection(mut stream: TcpStream, addr: SocketAddr) {
    // 该函数拥有这条连接
}
```

按值参数意味着 handler 取得所有权。所有权路径是：

```text
操作系统建立连接
  -> accept 构造并返回 TcpStream
  -> main 的 stream 绑定拥有它
  -> 函数调用移动给 handler 参数
  -> handler 返回，TcpStream drop
  -> Rust 关闭本地 socket handle
```

`mut stream` 中的 `mut` 属于参数绑定，不属于 `TcpStream` 类型。它允许 handler 以可变方式借用该绑定，例如调用 `Read::read(&mut self, ...)`。

原来的绑定不需要是 `mut` 才能移动：

```rust
let text = String::from("hello"); // 不可变绑定，但拥有 String
consume(text);                    // 移动所有权合法

fn consume(mut text: String) {
    text.push('!');
}
```

因此要分别问：

1. 谁拥有这个值？
2. 当前绑定能否被可变借用？
3. 是否有多个所有者？

这是三个不同的问题。

### 4.5 OS 线程是什么

`std::thread::spawn` 创建由操作系统调度的线程。每个线程通常拥有：

- 独立栈空间。
- 调度状态。
- 寄存器上下文。
- 内核用于管理线程的数据结构。

同一进程中的线程共享进程地址空间，所以它们理论上可以访问同一堆内存；Rust 再通过所有权、`Send`、`Sync` 和同步原语限制这种访问，防止数据竞争。

当连接线程阻塞在 socket `read()` 时，操作系统可以调度主线程或其他连接线程。socket 数据到达后，内核让等待线程重新变成可运行状态。

### 4.6 `thread::spawn` 的类型含义

标准库函数可以简化理解为：

```rust
pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
```

逐项理解：

- `F`：闭包的具体匿名类型。
- `FnOnce()`：闭包不接收参数，并且至少能被调用一次；它可能消耗捕获值。
- `T`：线程闭包最终返回的值。
- `F: Send`：闭包连同捕获状态能被转移到新线程。
- `F: 'static`：闭包不能依赖可能先失效的短生命周期借用。
- `T: Send + 'static`：线程返回值可能通过 `JoinHandle` 传回调用线程，所以也必须能够安全跨线程。

在当前项目中，闭包通常返回 `()`，关键约束集中在闭包捕获的 `TcpStream` 和 `SocketAddr` 上。

### 4.7 闭包可以理解成匿名结构体

下面的闭包：

```rust
let closure = || use_value(value);
```

概念上可以想象为编译器生成一个匿名结构体，把捕获的外部变量存成字段，再为它实现某个 `Fn` trait。实际捕获字段是引用还是拥有的值，取决于闭包如何使用变量以及是否带 `move`。

捕获模式大致有：

- `&T`：共享借用捕获。
- `&mut T`：可变借用捕获。
- `T`：按值捕获。

不同变量可以采用不同捕获方式。例如，把非 `Copy` 的 `stream` 传给按值参数会消耗它；读取一个 `Copy` 的 `addr` 可能只需要借用或复制。因此第一次不写 `move` 时，应阅读实际诊断，而不是假定两个变量必然产生相同错误。

### 4.8 为什么普通借用在线程中危险

考虑一个局部字符串：

```rust
fn start() {
    let text = String::from("hello");
    thread::spawn(|| println!("{text}"));
}
```

如果闭包借用 `text`，可能发生：

```text
start 返回
  -> text 被 drop
  -> 新线程之后才运行
  -> 访问已经失效的引用
```

操作系统不保证新线程一定在 `start` 返回前运行完。Rust 因而在编译期拒绝这种不受保证的借用。

### 4.9 `move` 真正做了什么

`move || { ... }` 要求闭包按值捕获使用到的外部变量。对于 `TcpStream`：

```text
main 拥有 TcpStream
  -> move closure 拥有 TcpStream
  -> closure 被送到新线程
  -> handler 从 closure 取得 TcpStream
```

移动不是深拷贝，也不会复制内核 socket。除非显式调用 `try_clone()`，这里仍然只有一个 Rust `TcpStream` 值沿所有权路径移动。

对于 `Copy` 类型，按值捕获可能产生按位复制，这是类型自身的 `Copy` 语义；不能把这个现象推广为“move 会复制所有值”。

### 4.10 `Send`：值能否跨线程移动

`Send` 是 unsafe auto trait。通常编译器根据类型的组成自动判断它是否实现 `Send`。

```text
T: Send
=> T 的所有权可以安全地转移到另一个线程
```

例子：

- `TcpStream` 可以安全转移，所以实现 `Send`。
- `String` 拥有自己的堆数据，可以转移，通常是 `Send`。
- `Rc<T>` 的引用计数更新不是原子的，不适合跨线程共享所有权，因此不是 `Send`。
- `Arc<T>` 使用原子引用计数，在满足内部类型约束时可以跨线程。

`move` 只决定如何捕获；它不能把一个非 `Send` 类型变成 `Send`。

### 4.11 `Sync`：共享引用能否跨线程

可以用一个关系帮助记忆：

```text
T: Sync
约等于
&T: Send
```

也就是说，如果 `T` 是 `Sync`，多个线程可以安全持有指向它的共享引用。

`Send` 和 `Sync` 的区别：

```text
Send：把所有权交给另一个线程
Sync：多个线程安全共享 &T
```

当前每条连接主要使用 `Send`：连接从主线程移动到连接线程。共享统计信息才会涉及 `Sync` 和同步原语。

### 4.12 `'static` 边界不等于永远存活

`F: 'static` 的重点是：闭包不能含有短生命周期引用。它不表示闭包对象实际存活到程序结束。

```rust
let owned = String::from("hello");
// owned 被 move 进闭包：闭包拥有数据，不依赖当前栈借用
```

线程可能 1 毫秒后结束，随后字符串正常释放。它依然可以满足 `'static` 类型边界。

需要区分：

- `T: 'static`：`T` 内部没有受限于较短生命周期的借用。
- `&'static T`：这个具体引用本身保证在整个程序期间有效。

二者不是同一句话。

### 4.13 `JoinHandle` 与线程结束

`spawn` 返回 `JoinHandle<T>`：

- `join()` 等待线程结束，并取得线程返回值或 panic 信息。
- 丢弃 handle 不会取消线程；线程会继续运行。
- 如果整个进程结束，其他线程也不会让进程继续提供服务。

在 accept 循环内立刻 join 会造成：

```text
spawn A -> main 等 A 结束 -> 仍无法 accept B
```

因此基础服务器暂时不立即 join。生产服务最终需要记录线程、限制数量并在关闭时协调退出，但那不是本阶段第一步。

### 4.14 `Arc<T>` 为什么存在

所有权移动适合“一条连接只交给一个线程”。但活跃连接数等状态需要被多个线程共同访问。

`Arc<T>` 提供线程安全的共享所有权：

```text
Arc::clone
  -> 增加原子引用计数
  -> 产生另一个指向同一 T 的 Arc handle
  -> 不深拷贝 T
```

当最后一个 `Arc` 被 drop，内部值才会被释放。

`Arc` 只解决“谁拥有数据”，没有自动提供内部可变性。`Arc<usize>` 不能直接被多个线程执行 `+= 1`。

### 4.15 `Mutex<T>` 如何保护共享修改

`Mutex<T>` 保证同一时刻只有一个线程持有锁：

```text
lock()
  -> 等待互斥锁
  -> 得到 MutexGuard
  -> 通过 guard 访问 T
  -> guard 离开作用域并 drop
  -> 自动解锁
```

组合成 `Arc<Mutex<usize>>` 后：

- `Arc` 让多个线程共同拥有 mutex。
- `Mutex` 让多个线程串行修改计数。

必须尽可能缩短锁的作用域。如果拿着全局锁调用阻塞式 `read()`，一个不发送数据的客户端会让所有需要同一锁的线程等待，等于重新引入全局阻塞。

### 4.16 Mutex poisoning

如果线程持有标准库 `Mutex` 时 panic，mutex 会被标记为 poisoned。后续 `lock()` 返回 `PoisonError`，提醒共享数据可能只修改了一半。

Poisoning 不是互斥锁失效，而是一种一致性警报。学习代码中常见 `.lock().unwrap()` 会在发现 poisoning 时继续 panic；真实系统需要根据状态不变量决定恢复还是终止。

### 4.17 锁、原子变量与连接计数

简单连接数也可以使用 `AtomicUsize`。两者侧重点不同：

- `Mutex<usize>`：通用、语义直观，适合先学习临界区。
- `AtomicUsize`：无需互斥锁，但需要理解原子操作和内存顺序。

Day 2 先使用 `Arc<Mutex<usize>>`，目的不是宣称它是计数器的最佳实现，而是学习共享所有权和互斥访问。

### 4.18 错误的故障范围

服务器需要区分两类错误：

```text
listener/bind/accept 级错误
  -> 可能影响整个服务

单连接 read/write 错误
  -> 应通常只结束该连接线程
```

如果连接线程因为写入失败而 panic，其他连接线程和 listener 通常仍可继续；但该线程的清理逻辑可能未执行。因此后续应把连接错误记录清楚并正常返回，而不是全部 `unwrap()`。

### 4.19 一连接一线程的扩展性成本

一连接一线程的优势是控制流直观：每个线程可以顺序编写阻塞代码。但它的成本与连接数绑定：

- 每个线程需要栈地址空间，实际物理内存按使用逐步提交，但并非零成本。
- 内核要保存和调度每个线程的状态。
- 可运行线程过多会增加上下文切换。
- 大量空闲连接对应大量等待线程。
- 无界 `spawn` 没有过载保护，可能耗尽线程、内存或文件描述符。
- 慢客户端可以长期占用连接线程。

线程池适合大量短任务，但对于“一条长连接在整个生命周期里长期阻塞”的模型，固定大小线程池会让长连接占满 worker，后续连接排队，因此不是最终解决方案。

这正是 Day 3/4 引入 Future 和 Tokio 的动机：等待 I/O 的连接不再需要永久占据一条专属 OS 线程。

## 5. 实践路线：接下来依次做什么

下面的实验用于验证上面的知识。先预测，再实现，再运行，最后解释；不要把它当成只需完成的 checklist。

## 6. 阶段一：提取单连接处理函数

### 6.1 为什么先提取函数

Day 1 的 `main` 同时负责：

```text
bind / listener 生命周期
accept 新连接
连接 buffer
read 循环
EOF 处理
write echo
```

更合理的职责边界是：

```text
main
  └── listener + accept

handle_connection
  └── 一条 TcpStream 从接管到关闭的生命周期
```

后面线程闭包只需要拥有连接并调用 handler。

### 6.2 所有权与 `mut`

不可变绑定仍然可以移动其拥有的值：

```rust
fn consume(mut value: String) {
    value.push('!');
}

let value = String::from("hello");
consume(value);
```

这里：

- `value` 拥有 `String`，但当前绑定不可变。
- 按值调用函数会移动所有权，不要求原绑定是 `mut`。
- 函数参数是一个新绑定，可以独立声明为 `mut`。
- 函数调用后，原来的 `value` 已失效。

对应到 TCP：

```text
main 的 stream（不需要 mut）
        ↓ move
handler 的 stream 参数（需要 mut 以调用 read）
```

### 6.3 动手任务

创建独立的 `week1/day2_thread_server`，保留 Day 1 代码作为对照。

handler 骨架：

```rust
fn handle_connection(/* stream */, /* addr */) {
    // buffer、read loop、EOF、echo
}
```

自行确定：

- 两个参数的类型。
- 按值还是引用。
- 哪个参数绑定需要 `mut`。

本阶段禁止添加 `thread::spawn`、`Arc` 和 `Mutex`。

### 6.4 预测问题

1. 调用 `handle_connection(stream, addr)` 后，main 能否再次使用 `stream`？
2. handler 阻塞在 `read()` 时，main 能否继续下一次 `accept()`？
3. 为什么 main 中的 `stream` 不需要 `mut`，handler 中却需要？
4. handler 返回时，`TcpStream` 会发生什么？

### 6.5 编译检查

```powershell
cargo check -p day2_thread_server
cargo fmt --check -p day2_thread_server
```

### 6.6 顺序行为实验

启动服务端：

```powershell
cargo run -p day2_thread_server
```

客户端 A：

```powershell
$clientA = [System.Net.Sockets.TcpClient]::new("127.0.0.1", 8080)
$streamA = $clientA.GetStream()
```

客户端 B：

```powershell
$clientB = [System.Net.Sockets.TcpClient]::new("127.0.0.1", 8080)
$streamB = $clientB.GetStream()
$dataB = [System.Text.Encoding]::UTF8.GetBytes("from B")
$streamB.Write($dataB, 0, $dataB.Length)
```

先观察服务端，不要立即在 B 中同步读取。关闭 A 后，再在 B 中读取：

```powershell
$clientA.Close()
$bufferB = New-Object byte[] 1024
$nB = $streamB.Read($bufferB, 0, $bufferB.Length)
[System.Text.Encoding]::UTF8.GetString($bufferB, 0, $nB)
$clientB.Close()
```

记录：B 是否完成连接、服务端何时打印 B、B 何时收到 Echo，以及实际日志顺序。

## 7. 阶段二：第一次 `thread::spawn` 实验

### 7.1 线程解决的问题

普通调用：

```text
main -> handler A -> read A 阻塞 -> 无法 accept B
```

线程版本：

```text
main -> spawn handler A -> main 回到 accept B
                         handler A 在线程 A 阻塞
```

`read()` 仍然阻塞，只是不再阻塞负责 accept 的主线程。

### 7.2 `thread::spawn` 模型

```rust
std::thread::spawn(|| {
    // 新 OS 线程运行的代码
});
```

它返回一个 `JoinHandle`。闭包可能比当前循环迭代甚至当前函数活得更久，因此不能随意借用当前栈帧中的局部数据。

### 7.3 闭包捕获

闭包针对每个外部变量，可能使用：

- 共享借用捕获。
- 可变借用捕获。
- 按值捕获。

捕获方式根据闭包内部的使用方式推断。不同变量可能采用不同方式。不能简单背诵“闭包默认全部借用”。

### 7.4 故意不写 `move`

将直接调用 handler 改为：启动线程，在闭包中调用 handler。第一次不要写 `move`，然后运行：

```powershell
cargo check -p day2_thread_server
```

不要马上修复。保存完整诊断并回答：

1. 错误涉及 `stream`、`addr`，还是其中之一？
2. `stream` 被按什么方式使用？
3. `SocketAddr` 是 `Copy` 是否影响捕获推断？
4. 编译器在防止哪一种悬垂引用？

实际错误信息比预想结果更重要。

## 8. 阶段三：`move`、`Send` 和 `'static`

### 8.1 `move` 闭包

```rust
move || {
    // 闭包按值捕获使用到的外部变量
}
```

`move` 表示按值捕获，不表示：

- 深拷贝值。
- 复制 socket 内核对象。
- 自动实现线程安全。
- 自动共享值。

本项目的所有权路径应是：

```text
accept 返回 TcpStream
  -> main 的 stream
  -> move closure
  -> connection thread
  -> handle_connection
  -> handler 返回后 drop
```

### 8.2 `Send`

`Send` 表示一个值的所有权可以安全转移到另一个线程。

线程闭包捕获 `TcpStream` 和 `SocketAddr`，这些捕获值必须能够跨线程转移，闭包本身才可以被交给新线程。

`Send` 不表示多个线程可以同时共享该值。共享引用能否跨线程更接近 `Sync` 的含义。

### 8.3 `'static`

`thread::spawn` 要求闭包满足 `'static` 边界。这里的含义不是“值永远存在”，而是闭包不能包含依赖当前短生命周期的借用。

一个拥有 `String`、`TcpStream` 等数据的闭包可以满足 `'static`，即使线程只运行几毫秒；线程结束时数据仍会正常 drop。

### 8.4 预测问题

1. 加入 `move` 后，主线程还能使用原 `stream` 吗？
2. 是否有两个线程同时拥有同一个 `TcpStream` 值？
3. 是否调用了 `TcpStream::try_clone()`？
4. 非 `Send` 类型使用 `move` 后能否发送到新线程？
5. 为什么按值移动 `String` 通常满足 `'static`，借用局部 `&String` 通常不满足？

### 8.5 修复和检查

解释第一次错误后，再调整闭包所有权并运行：

```powershell
cargo check -p day2_thread_server
cargo fmt --check -p day2_thread_server
```

当前不要添加 `Arc<Mutex<TcpStream>>`。每条连接应由对应线程独占。

## 9. `JoinHandle` 与为什么不能立即 `join`

`thread::spawn` 返回 `JoinHandle`。`join()` 会阻塞调用线程，直到目标线程结束。

如果在 accept 循环中立即 join：

```text
spawn A
  -> main join A
  -> A 阻塞在 read
  -> main 仍然不能 accept B
```

这会破坏一连接一线程模型的目标。基础版本可以暂时丢弃 handle，连接线程仍继续运行。后续讨论服务关闭时，再设计线程跟踪和回收。

问题：立即 join 与直接调用 handler，在并发行为上有什么本质区别？

## 10. 阶段四：并发行为验证

### 10.1 实验步骤

1. 启动线程版本服务端。
2. 客户端 A 连接并保持空闲。
3. 客户端 B 连接、发送 `from B` 并读取 Echo。
4. A 不关闭的情况下，确认 B 仍然立即获得 Echo。
5. A 再发送 `from A` 并读取 Echo。
6. 分别关闭 A、B，观察连接线程退出。

### 10.2 线程日志

可以在日志中加入当前线程 ID 以辅助观察；先查阅 `std::thread::current()` 和 `Thread::id()`，不要复制完整实现。

预期结构：

```text
main thread: accept A
thread X: read A
main thread: accept B
thread Y: read B
```

### 10.3 需要回答

1. A 的空闲 `read()` 阻塞了哪个线程？
2. 哪个线程继续调用 `accept()`？
3. A 和 B 的日志为什么可能交错？
4. 日志先后顺序能否证明网络数据的全局顺序？
5. handler 返回时，socket 和 OS 线程分别发生什么？

## 11. 阶段五：共享状态与 `Arc<Mutex<T>>`

一连接一线程本身不要求共享 `TcpStream`。但真实服务器常需要共享：

- 活跃连接数。
- 全局配置。
- 统计信息。
- 共享缓存。

本阶段使用“活跃连接数”作为最小实验，学习 `Arc<Mutex<T>>`，不把它用于连接本身。

### 11.1 为什么只有 `Mutex<T>` 不够

如果在 accept 循环中把同一个 `Mutex<usize>` 按值移动给第一个线程，主线程之后就不能把它交给第二个线程。

多个线程需要共同拥有同一份状态，因此需要共享所有权。

### 11.2 `Arc<T>`

`Arc<T>` 是原子引用计数智能指针：

- 多个线程可以各自拥有一个 clone 后的 `Arc` handle。
- `Arc::clone` 增加引用计数，不会深拷贝内部数据。
- 最后一个 `Arc` 被 drop 时，内部数据被释放。

`Arc<T>` 只解决共享所有权，不自动允许安全修改内部值。

### 11.3 `Mutex<T>`

`Mutex<T>` 通过互斥锁限制同一时刻只有一个线程访问内部值：

```text
lock
 -> 获得 MutexGuard
 -> 访问/修改数据
 -> guard drop
 -> unlock
```

不要在持锁期间进行阻塞网络 I/O，否则一个慢客户端可能让其他线程无法访问共享状态。

### 11.4 组合

```text
Arc<Mutex<usize>>
│   │     └── 活跃连接数
│   └── 互斥修改
└── 多线程共享所有权
```

### 11.5 动手实验

在 main 创建共享连接计数。每次 accept 后，为连接线程 clone 一个 `Arc` handle：

```text
连接线程开始
 -> lock
 -> count += 1
 -> 尽快释放锁
 -> handle_connection
 -> lock
 -> count -= 1
 -> 尽快释放锁
```

本阶段关注基本语义。异常退出时自动递减和 RAII guard 可以作为后续改进，不要求立即设计完整连接守卫。

### 11.6 预测问题

1. 为什么 `Arc::clone` 不是复制计数值？
2. 为什么 `Arc<usize>` 不能直接安全地执行 `+= 1`？
3. 为什么只有 `Mutex<usize>` 不能方便地被多个线程长期共同拥有？
4. 如果在持锁时调用 `stream.read()`，其他连接线程会发生什么？
5. 某线程持锁时 panic，其他线程再次 `lock()` 可能观察到什么？

### 11.7 关于原子类型

简单计数也可以使用 `Arc<AtomicUsize>`，但本单元先用 `Arc<Mutex<usize>>` 学习通用共享状态。之后再比较原子操作的内存顺序和适用范围。

## 12. 错误处理检查

当前 Day 1 风格可能包含：

```rust
while let Ok(n) = stream.read(...)
stream.write_all(...).unwrap()
```

它的问题是：

- read error 会静默退出，没有日志。
- 单连接 write error 会 panic 当前连接线程。
- panic 不会直接让其他连接线程 panic，但会失去本线程的清理逻辑。

Day 2 至少应区分：

```text
Ok(0)     -> orderly EOF
Ok(n)     -> echo n bytes
Err(err)  -> 记录连接级错误并结束该连接
```

不要求现在引入复杂错误库。目标是认识“某条连接失败”和“整个监听服务失败”属于不同故障范围。

### 12.1 有序 EOF 与连接复位

TCP 连接结束不只有 `read() -> Ok(0)` 一条路径。

**有序关闭**通常表现为：对端发送 FIN，服务端先读取完已经到达的所有字节，随后一次 `read()` 返回 `Ok(0)`。这表示对端不会再发送新字节。

**异常/复位关闭**可能表现为：对端发送 RST，或者操作系统把当前关闭解释为中止连接。服务端的 `read()` 会返回 `Err`，常见 `ErrorKind` 是 `ConnectionReset`。

在 Windows/.NET TCP 客户端实验中，如果服务端已经回写 Echo，但客户端没有读取接收缓冲区中的 Echo 就直接 `Close()`，关闭行为可能导致连接复位，而不是服务端观察到单纯的 FIN。因此可能出现：

```text
客户端读取 Echo 后 Close
  -> 服务端下一次 read 得到 Ok(0)
  -> 打印 orderly disconnected

客户端不读取 Echo 就 Close
  -> 服务端下一次 read 得到 Err(ConnectionReset)
  -> 若代码使用 while let Ok(...)，错误被静默丢弃
```

这不是“客户端必须读取才能关闭 TCP”，而是未读取的已到达数据会影响具体关闭路径。行为还可能受操作系统、socket 选项和关闭时序影响，所以应以服务端记录的真实 `io::ErrorKind` 为准。

### 12.2 为什么 `while let Ok(n)` 隐藏了事实

```rust
while let Ok(n) = stream.read(&mut buffer) {
    // ...
}
```

它等价于只处理 `Ok` 分支：一旦返回 `Err`，循环直接结束，错误值没有被绑定、记录或分类。因此“没有打印 disconnected”不代表连接线程仍在运行；它可能已经通过错误路径退出。

更适合网络服务的控制流形状是：

```rust
loop {
    match stream.read(&mut buffer) {
        Ok(0) => {
            // TODO: 记录有序 EOF，然后结束连接
        }
        Ok(n) => {
            // TODO: 处理并回写有效字节；写错误也属于连接级错误
        }
        Err(error) => {
            // TODO: 记录 error 与 error.kind()，然后结束连接
        }
    }
}
```

先由学习者补全各分支，不直接复制最终实现。

### 12.3 对照实验

运行两组客户端：

1. 发送 -> 读取完整 Echo -> Close。
2. 发送 -> 不读取 Echo -> Close。

服务端改为记录 `read` 错误后，比较：

- 第一组最后一次 `read()` 的结果。
- 第二组最后一次 `read()` 的结果与 `ErrorKind`。
- `write_all` 是否也可能先观察到错误。
- 连接线程最终是否都退出。

预测问题：如果客户端只读取一部分 Echo 后关闭，服务端更可能观察到 EOF 还是复位？为什么这个答案不应只靠语言规范猜测，而应通过操作系统实验验证？

### 12.4 服务端知道“连接复位”，不知道“应用是否调用了 Read”

必须区分三个不同的完成层次：

```text
服务端 write_all 返回成功
  -> 字节已经交给服务端本地 TCP 栈

服务端收到 TCP ACK
  -> 客户端 TCP 栈已经接收字节并放入内核接收缓冲区

客户端应用调用 Read
  -> 字节才从内核缓冲区复制/交付给客户端应用
```

前两步都不能证明客户端应用已经读取或处理数据。普通 TCP 没有“对端应用已经消费这条业务消息”的确认语义。

如果客户端在接收缓冲区仍有未被应用读取的数据时关闭 socket，某些操作系统/时序会发送 RST，表示连接被中止，未读数据被丢弃。服务端能观察到的是：

```text
read -> Err(ConnectionReset)
```

服务端不能仅凭这个错误准确反推出客户端应用执行了哪一行代码。RST 还可能由进程崩溃、网络设备、socket 选项或其他异常产生。

如果业务必须确认客户端确实处理了消息，就要在 TCP 之上设计应用层 ACK，例如：

```text
Server -> EVENT 42
Client -> ACK 42
```

即使收到应用层 ACK，也只能证明客户端按照协议声称已处理到该阶段；数据库提交、幂等和故障恢复仍需由应用协议定义。

### 12.5 FIN、RST 与半关闭

TCP 报文头中包含多个控制标志。连接结束时最重要的是 FIN 和 RST。

**FIN（Finish）**表示发送方有序关闭自己的发送方向：

```text
“我不会再发送新字节；此前发送的字节仍然有效，请按顺序读取。”
```

接收方读完 FIN 之前到达的数据后，`read()` 返回 `Ok(0)`。FIN 只关闭一个方向，所以 TCP 支持半关闭：客户端可以不再发送，但仍继续接收服务端数据。

**RST（Reset）**表示立即复位/废弃连接：

```text
“这条连接不能再按正常流程继续；不要等待剩余数据或正常关闭握手。”
```

RST 通常由操作系统 TCP 栈生成，而不是应用手工构造。收到 RST 后，未完成的数据和正常 EOF 语义不再可靠；阻塞的读写通常以 `ConnectionReset` 等错误结束，内核可以立即删除该连接状态。

简化比较：

| 项目 | FIN | RST |
|---|---|---|
| 语义 | 有序结束发送方向 | 立即中止整条连接 |
| 已发送数据 | 仍应按 TCP 顺序交付 | 可能被视为未可靠完成/被丢弃 |
| 对端读取表现 | 最终 `Ok(0)` | 通常 `Err(ConnectionReset)` |
| 是否支持半关闭 | 是 | 否 |
| 常见来源 | 正常 shutdown/close | 未读数据关闭、进程异常、abortive close、协议状态异常 |

调用 `Close()` 是应用向操作系统表达“关闭 socket”；最终发送 FIN 还是 RST，要看当前 socket 状态、未读/未发送数据、linger 选项、操作系统实现和时序，不能把 `Close()` 简单等同于某一个 TCP 标志。

## 13. 阶段六：一连接一线程的成本

该模型解决：

- A 的阻塞读取不再阻塞 accept。
- 多个客户端可以并发获得服务。
- 代码控制流直观，每条连接顺序执行。

它没有解决：

- 每条连接占用一个 OS 线程。
- 每个线程需要栈空间和内核调度资源。
- 大量线程带来上下文切换。
- 大量空闲连接仍占据线程。
- 无限制 spawn 缺少过载保护。
- 恶意慢客户端可以长期占用资源。
- 服务关闭时如何通知和等待所有连接线程。

### 安全的小规模实验

逐步创建少量空闲连接，例如 10、50、100 个，观察：

- 进程线程数。
- 内存变化。
- 服务是否仍能快速接受新连接。
- 关闭客户端后线程数如何变化。

不要为了追求数字在本机无上限创建线程。实验目标是观察趋势，而不是压垮系统。

### 思考问题

1. 1000 个空闲连接为什么会对应大量空闲线程？
2. 空闲线程没有执行用户代码，为什么仍有成本？
3. 线程池能否直接解决长连接中长期阻塞的问题？
4. 我们真正希望的是“为每条连接提供并发执行”，还是“为每条连接创建线程”？
5. Future/Tokio 接下来试图解决哪一类资源问题？

## 14. 常见错误

### 错误：main 的 stream 保留不必要的 `mut`

main 只移动值；handler 的参数绑定才需要可变借用。

### 错误：认为 `move` 会复制 socket

移动所有权不等于调用 `try_clone()`。

### 错误：spawn 后立即 join

主线程重新等待连接线程，失去并发接受能力。

### 错误：用 `Arc<Mutex<TcpStream>>` 包装每条连接

当前连接只有一个线程处理，不需要共享；共享只会增加复杂度和锁。

### 错误：持有全局锁进行 `read()`/`write_all()`

阻塞 I/O 会扩大临界区，使其他线程等待。

### 错误：把 `Send` 理解成可同时共享

`Send` 是所有权跨线程转移；`Sync` 才与共享引用跨线程相关。

### 错误：把 `'static` 理解成永不释放

它限制借用关系，不要求实际运行时间无限长。

## 15. 分级提示

### Level 1：概念

主线程只负责 accept；连接线程拥有 stream 并运行 handler。

### Level 2：API

查看：

- `std::thread::spawn`
- `std::sync::Arc`
- `std::sync::Mutex`
- `std::thread::current`

### Level 3：结构

```text
accept stream
 -> clone shared counter handle
 -> spawn move closure
      -> update count
      -> handle_connection(stream, addr)
      -> update count
```

### Level 4：局部帮助

只在你已经尝试并提供编译错误后，针对闭包捕获、`Arc::clone` 或锁 guard 的具体一处给代码提示。

## 16. Day 2 最终解释题

1. Day 1 为什么不能同时处理 A 和 B？
2. 一连接一线程具体把哪个阻塞点移到了哪里？
3. `move`、`Send` 和 `'static` 分别解决什么问题？
4. `Send` 与 `Sync` 有什么区别？
5. 为什么 main 中的 `stream` 不需要 `mut`？
6. 为什么不能立即 join？
7. 为什么连接不需要 `Arc<Mutex<TcpStream>>`？
8. `Arc` 与 `Mutex` 各自解决什么问题？
9. 为什么不能持锁执行阻塞 I/O？
10. handler 返回时，socket、闭包捕获值和 OS 线程如何结束？
11. 该模型支持多个客户端，为什么仍不适合无限扩展？
12. Tokio 下一步要解决的核心限制是什么？

## 17. 完整验收标准

只有全部满足才完成 Day 2：

- handler 拥有 `TcpStream`，Day 1 行为没有退化。
- 保存并解释过一次不写 `move` 的真实编译结果。
- 线程版本通过 `cargo check` 和 `cargo fmt --check`。
- A 保持空闲时，B 仍能及时获得 Echo。
- 能通过线程 ID 或日志解释并发执行。
- 能解释 `move`、`Send`、`Sync` 和 `'static`。
- 完成一个小型 `Arc<Mutex<usize>>` 共享计数实验。
- 没有在持锁期间执行连接 I/O。
- read error 不再完全静默，连接错误不会停止 listener。
- 能说明 join、线程生命周期和 socket drop。
- 完成一次有限的空闲连接资源观察。
- 独立回答最终解释题。

## 18. 推荐资料

### 必读

1. [《Rust 程序设计语言》：使用线程同时运行代码](https://rustwiki.org/zh-CN/book/ch16-01-threads.html)

   重点：`thread::spawn`、`JoinHandle`、`move` 闭包。

2. [`std::thread` 官方文档](https://doc.rust-lang.org/std/thread/)

   重点：线程模型、spawning、线程结束与 join。

3. [《Rust 程序设计语言》：使用消息传递在线程间传送数据](https://rustwiki.org/zh-CN/book/ch16-02-message-passing.html)

   当前只需理解所有权跨线程转移的背景，不要求本 Day 实现 channel。

4. [《Rust 程序设计语言》：共享状态并发](https://rustwiki.org/zh-CN/book/ch16-03-shared-state.html)

   重点：`Mutex<T>` 与 `Arc<T>` 为什么组合使用。

5. [《Rust 程序设计语言》：`Sync` 与 `Send`](https://rustwiki.org/zh-CN/book/ch16-04-extensible-concurrency-sync-and-send.html)

### 选读

6. [Rustonomicon：Send and Sync](https://doc.rust-lang.org/nomicon/send-and-sync.html)

   只用于深化理解；当前不要尝试手动 `unsafe impl Send/Sync`。

## 19. 实验记录模板

```text
阶段：

运行前预测：

编译器/运行结果：

结果与预测是否一致：

所有权路径：

发生阻塞的线程：

关键错误及原因：

我能独立解释的内容：

仍不清楚的问题：
```

## 20. 下一单元衔接

Day 3 不会因为“线程不好”就直接背 Tokio API。它从这个已经观察到的限制出发：

```text
大量连接
  -> 大量线程
  -> 大量空闲栈与调度成本
  -> 希望等待 I/O 时不占用一条专属线程
  -> Future / Pending / Waker / executor
```

进入 Day 3 前，你必须能够回答：

> 我们试图消除的是阻塞 I/O 本身，还是“一个等待中的连接永久占用一条 OS 线程”的资源模型？
