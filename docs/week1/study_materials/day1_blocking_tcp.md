# Day 1：阻塞式 TCP Echo Server

## 今天学什么

今天使用 Rust 标准库写一个最小 TCP Echo Server：客户端发送什么字节，服务端就回写什么字节。这个程序一次只能主动处理一个连接，正是明天学习线程并发的出发点。

## 核心对象

```rust
let listener = TcpListener::bind("127.0.0.1:8080")?;
let (mut stream, addr) = listener.accept()?;
```

- `TcpListener` 是监听某个 IP 和端口的服务端 socket。
- `accept()` 在没有连接时阻塞；连接到来后返回该连接的 `TcpStream` 和对端地址。
- `TcpStream` 代表一条已建立的 TCP 连接，可从中读取、向其中写入。

## `read()` 与有效数据

```rust
let mut buffer = [0u8; 1024];
let n = stream.read(&mut buffer)?;
```

`n` 是这一次实际读取的字节数。有效数据只有：

```rust
&buffer[..n]
```

例如客户端发送 `hello` 时，`n` 通常是 5。缓冲区剩余位置不是本次消息的一部分：首次读取时通常是 `0`，重复使用缓冲区时还可能保留前一次读取留下的字节。回写整个 1024 字节数组会把无效内容一同发送。

## Echo 的最小写入操作

需要引入 `Write` trait：

```rust
use std::io::{Read, Write};

// n 是本次 read 得到的长度
stream.write_all(&buffer[..n])?;
```

`write_all` 会反复写入，直到整个切片都已交给操作系统，或遇到错误。它适合本练习，因为 Echo 必须回写完整的有效切片。

## 数据怎样到达客户端

```text
服务端 Rust 程序
  write_all(&buffer[..n])
        ↓
服务端内核 TCP 发送缓冲区
        ↓
网络
        ↓
客户端内核 TCP 接收缓冲区
        ↓
客户端程序 Read(...)
```

`write_all` 成功时，首先表示服务端操作系统接管了这些字节；不能据此断言客户端程序此刻已经读取到它们。TCP 之后负责可靠、有序地传送数据。

## 断开连接与 `read() == 0`

当客户端正常关闭其连接，并且服务端已经读完此前收到的字节后：

```rust
let n = stream.read(&mut buffer)?;
// n == 0
```

零字节表示对端不会再发送数据（EOF）。它不是错误，但必须结束当前连接的处理循环。若继续循环读取，会持续得到 `0` 并造成空转。

```text
客户端发送 hello → 服务端回显 hello → 客户端关闭
                                      ↓
                         服务端下一次 read() 返回 0
```

服务端关闭 socket 后，客户端仍能读到已经按序抵达的回写数据；读完后再次读取才会看到 EOF。

## 两层循环的意义

```text
外层：accept 一个客户端
内层：持续 read / echo，直到 n == 0
```

这样服务端可以在客户端 A 断开后回到 `accept()`，再处理客户端 B。因此它能**依次**服务多个客户端。

但它无法**同时**服务多个客户端：如果服务端正在客户端 A 的 `read()` 上等待数据，唯一的线程无法返回 `accept()` 来取得并处理客户端 B。

## Windows 手动测试

服务端运行：

```powershell
cargo run -p day1_tcp_echo
```

另一个 PowerShell 窗口可使用内置的 .NET 客户端：

```powershell
$client = [System.Net.Sockets.TcpClient]::new("127.0.0.1", 8080)
$stream = $client.GetStream()
$bytes = [Text.Encoding]::UTF8.GetBytes("hello")
$stream.Write($bytes, 0, $bytes.Length)

$buffer = New-Object byte[] 1024
$n = $stream.Read($buffer, 0, $buffer.Length)
[Text.Encoding]::UTF8.GetString($buffer, 0, $n)

$client.Close()
```

预期客户端打印 `hello`，服务端打印收到的 5 个字节并在客户端关闭后结束当前连接处理。

## 自测问题

1. 为什么 TCP 中一次 `send` 不保证对应服务端的一次 `read`？
2. 为什么 `&buffer[..n]` 比 `&buffer` 正确？
3. `read() == 0` 和 I/O 错误有什么区别？
4. 为什么单线程服务端不能同时处理空闲的客户端 A 和发送消息的客户端 B？
