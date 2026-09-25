# 学习主题：Future、poll、Waker 与最小执行器

> 这是一个完整的学习主题，不按自然日划分。可以分多次完成；每次从上次的实验记录继续。
> 当前先使用标准库，不引入 Tokio。请按“预测 → 实现 → 运行 → 解释”的顺序完成练习。

## 1. 为什么现在学它

在线程版 TCP Echo 中，每个连接有自己的 OS 线程。客户端不发数据时，该线程阻塞在 `read()`；主线程仍可接受其他连接，但每个空闲连接依然占有一个线程。我们想保留“等待 A 时还能处理 B”的能力，同时不要求每个等待中的连接都占着一个 OS 线程。

Future 是描述“可能还没完成的计算”的 Rust 值。它允许一次尝试暂时结束，把执行线程还给调度者；条件变化后，调度者再尝试推进它。**Future 本身不负责检测 socket 是否可读，也不自动创建线程。** 本主题只建立执行模型，不把 Day 2 服务端改成异步服务器；网络 I/O 将在后续主题中接上。

### 完成后应能解释

1. 创建 Future、轮询 Future、完成 Future 分别是什么。
2. `poll` 为什么返回 `Pending` 或 `Ready`，以及为什么它应尽快返回。
3. `Waker` 通知了谁；`wake()` 与再次 `poll()` 是什么关系。
4. 谁负责保存、调度和再次轮询 Future。
5. 为什么“在 `poll` 里阻塞”或“无条件反复轮询”都没有解决线程版的问题。
6. `Pin<&mut Self>` 在接口中保证什么；当前实验为什么可以用 `get_mut()`。

## 2. 前置知识与边界

你已经完成阻塞 TCP Echo、一连接一线程，以及 `move`、`Send`、共享计数的练习。这里需要会写结构体、`impl`、枚举匹配和 `Result`，不需要事先掌握 Tokio。

本主题使用一个**人为控制的状态**演示调度；它不是真实的非阻塞网络驱动。也不要求实现通用、多任务、生产级执行器。不要提前引入 Tokio、`futures` crate、`RawWaker` 或 `unsafe`。

## 3. 知识模型

### 3.1 Future 是值，创建不等于执行

`async fn` 被调用时，返回的是一个实现 `Future` 的值。函数体的工作由后续轮询推进。一个 Future 如果从未被轮询，其内部工作通常不会发生。`.await` 会在另一个 Future 中等待并推进它，但最外层仍需要执行器驱动。

对照线程：`thread::spawn` 会创建可由 OS 调度的线程；单独创建 Future 不会发生这种调度。这里说的是 Future 自身的执行；外部已经启动的其他线程或 I/O 不受这一点限制。

### 3.2 `Future::poll` 的契约

接口的核心形状是：

```rust
trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}
```

`Poll::Ready(value)` 表示已经得到最终结果。执行器应停止轮询这个 Future；完成后再次轮询没有通用行为保证。`Poll::Pending` 表示**这次**还无法完成，调用方现在应去做别的事，而 Future 需要安排在条件变化时唤醒对应任务。`Pending` 不等于“永远失败”或“自动休眠一段时间”。

`poll` 应迅速返回。若它直接调用会长期阻塞的 `std::thread::sleep` 或阻塞式 `TcpStream::read`，执行器线程仍被占住；把函数签名写成 `poll` 并不会让操作自动变成非阻塞。

### 3.3 `Context`、`Waker` 与重新调度

执行器调用 `poll` 时提供 `Context`，其中的 `Waker` 标识**当前任务**。当 Future 暂时无法推进时，它要确保将来有条件继续时能通知这个任务。典型做法是保存 `cx.waker().clone()`；条件变化后调用 `wake()`。

```text
执行器 poll Future
        ↓
Future 返回 Pending，并安排将来 wake
        ↓
执行器去处理其他任务或等待通知
        ↓
事件发生 → wake → 任务被安排再次 poll
        ↓
Future 再次检查状态 → Pending 或 Ready
```

`wake()` **不是直接调用 `poll()`，也不保证下一次轮询一定 `Ready`**。它告诉执行器：这个任务现在值得再次尝试。若条件尚未满足，下一次仍可返回 `Pending`。如果每次轮询都更换了 Waker，Future 应以最近一次 `Context` 的 Waker 为准。

### 3.4 执行器与事件来源各做什么

最小执行器至少要持有 Future、发起首次轮询、接收唤醒通知，并在通知后再次轮询，直到 `Ready`。真实异步网络还需要 I/O 驱动：向 OS 注册 socket 的可读/可写兴趣，收到就绪事件时唤醒相关任务。Future、执行器、I/O 驱动是相关但不同的部分。

不要写成 `loop { future.poll(...) }`：如果一直返回 `Pending`，这会持续消耗 CPU。唤醒机制的意义是让执行器知道**什么时候再试**。

### 3.5 为什么接口有 `Pin<&mut Self>`

某些 Future 在暂停后可能在自身内部形成依赖地址的状态；如果这时随意移动它，会使这些状态失效。`Pin` 给这类 Future 提供“被固定后不再随意移动”的约束。当前的简单计数 Future 只含普通数字，可作为 `Unpin` 类型使用 `Pin::get_mut()`；本主题不要求手写自引用 Future，也不要求绕开 `Pin`。

## 4. 实验路线

在工作区创建一个独立 crate，例如 `week1/future_poll_lab`。名称按**内容**命名，不按 Day 编号。每完成一阶段，记录预测、实际输出和解释；上一阶段不清楚时不要直接跳到下一阶段。

### 阶段 A：观察“创建不执行”

写一个很小的 `async fn`：函数体开头打印一行，再返回一个整数。在 `main` 中先只调用它并保存返回的 Future，不使用执行器。

**运行前预测：**

1. 调用 `async fn` 的那一行是否打印函数体内的消息？
2. 立即 `drop` 这个 Future 会打印消息吗？
3. 为什么这个行为和直接调用普通函数不同？

运行并记录结果。不要为了让它运行而立即加入 Tokio；这个阶段的目标就是看到“值已经创建，但工作尚未推进”。

### 阶段 B：手写一个很小的 Future

定义 `CountDown { remaining: usize }`，实现 `Future<Output = &'static str>`。每次 `poll` 只做一小步：未结束时减少 `remaining`，返回 `Pending`；到达终点时返回 `Ready("done")`。先画出 `remaining = 2` 的状态变化，再写代码。可以先不写 Waker 或执行器，但此时它尚不满足一个真正可等待的 `Pending` 契约；阶段 C 会补上。

需要自行查阅并使用：`std::future::Future`、`std::pin::Pin`、`std::task::{Context, Poll}`。在这个简单类型上，想一想为什么可以通过 `Pin::get_mut()` 修改 `remaining`。

**运行前预测：** 第一次、第二次、第三次轮询分别返回什么？注意你在代码中选择的是“先判断再递减”还是“先递减再判断”；以实际实现画状态表，不靠猜测。

### 阶段 C：给 `Pending` 配上唤醒通知

使用标准库的 `std::task::Wake` trait，构造一个会向 `std::sync::mpsc` 通道发送通知的 Waker。让 `CountDown` 在返回 `Pending` 前调用 `cx.waker().wake_by_ref()`，表示它可以再推进一步。这是**人为的立即唤醒**，只用来观察调度协议；真实 I/O 应等到事件就绪才唤醒。

先用单个 Future 写一个最小驱动循环：首次 `poll` 由驱动者主动发起；若返回 `Pending`，等待通道通知，再轮询；若返回 `Ready`，打印结果并结束。用 `Box::pin` 固定 Future，避免手工处理不安全的 Pin 操作。

**实现前先画数据流：**

```text
驱动循环 ──poll──> CountDown
    ↑                  │
    └──通道通知 <── Waker
```

**运行前预测：**

1. 谁发起第一次 `poll`？
2. `wake_by_ref()` 是否直接执行下一次 `poll`？
3. 如果 `CountDown` 返回 `Pending` 却没有发出唤醒通知，驱动循环会停在哪里？
4. 如果驱动者收到通知后不再 `poll`，Future 会自己完成吗？

这一步不需要支持多个 Future、任务队列或跨线程 I/O。若 `Wake` 的 trait 方法或 `Context::from_waker` 的类型报错，先读编译错误和标准库签名，再寻求提示。

### 阶段 D：解释与 Day 2 的连接

无需再写网络服务器。画出两条空闲 TCP 连接在当前线程版中的等待位置，再画出理想的异步版本：任务在不能继续时返回 `Pending`，执行器线程去处理其他任务；socket 可读后，I/O 驱动唤醒相应任务。明确这只是概念图，下一主题才把真实 I/O 接进来。

## 5. 运行与记录

从工作区根目录运行（crate 名若不同，替换为自己的名称）：

```text
cargo check -p future_poll_lab
cargo run -p future_poll_lab
cargo fmt -p future_poll_lab -- --check
```

每阶段在自己的笔记或本文件末尾记录：

```text
预测：
实际观察：
预测与结果的差异：
原因：
下一阶段要解决的问题：
```

不需要为简单状态机写只重复实现逻辑的测试。若要验证行为，可用一次明确的状态序列检查：`Pending` 的次数、通知的次数、最终只得到一次 `Ready`，并确认驱动循环不会空转。

## 6. 常见错误与分级提示

| 现象 | 先检查什么 |
| --- | --- |
| 创建 Future 后没有输出 | 它是否曾被轮询？ |
| `poll` 返回 `Pending` 后程序永远等待 | 是否安排了唤醒？通知是否送到驱动循环？ |
| CPU 持续升高 | 是否无条件在循环中反复 `poll`？是否无条件反复唤醒却不能推进？ |
| `Pin<&mut Self>` 不能直接取 `&mut Self` | 当前类型是否 `Unpin`？应使用哪个安全 API？ |
| `Ready` 后行为怪异 | 是否错误地再次轮询已完成 Future？ |

求助时按下面顺序增加提示，不直接索取完整实现：

1. **概念：** 说明卡在“创建、首次轮询、Pending、通知、再次轮询”的哪一环。
2. **API：** 查看 `Future::poll`、`Poll`、`Wake`、`Waker`、`Context::from_waker`、`Box::pin` 的签名。
3. **结构：** 分开写 Future 的状态变化和驱动者的等待循环；用通道连接两者。
4. **局部代码：** 只展示出现编译或调度问题的几行，解释原因后自行修复。

## 7. 完成证据与复盘题

本主题完成需有以下证据：

- 自己实现并运行阶段 A、B、C；能指出每次 `poll` 的返回值与状态变化。
- `Pending` 后靠 Waker 通知触发下一次轮询，而非忙循环。
- 能解释 Future、任务、执行器和 I/O 驱动各自的职责。
- 能解释为什么当前 CountDown 的立即唤醒只是教学模拟，不能直接替代网络就绪事件。
- 能不用笔记回答下面的问题。

复盘题：

1. `async fn` 调用返回什么？谁让它开始执行？
2. `Pending` 之后，谁应该负责使任务有机会被再次轮询？
3. `wake()` 与 `poll()` 是同一件事吗？
4. 如果一次唤醒后再次得到 `Pending`，是否一定是 bug？
5. 在 `poll` 中使用阻塞 `read()` 会让哪个线程等待？
6. 线程版服务端的“每连接一线程”限制，异步模型打算怎样缓解？
7. 为什么本实验可以用 `Pin::get_mut()`，而不能据此认为所有 Future 都能随意移动？

## 8. 参考资料

**必读，按当前阶段阅读：**

1. [Rust 标准库 `Future` 文档](https://doc.rust-lang.org/std/future/trait.Future.html)：重点读 `poll` 的返回值、唤醒责任和运行特点；阶段 B 前阅读。
2. [Rust Async Book：Task Wakeups with `Waker`](https://rust-lang.github.io/async-book/02_execution/03_wakeups.html)：重点看 `Pending` 后如何通知；阶段 C 前阅读。示例使用的计时线程只是教学手段。

**选读：**

3. [The Rust Programming Language：异步编程](https://doc.rust-lang.org/book/ch17-00-async-await.html)：回顾线程与异步模型的关系。
4. [Rust Async Book：Build an Executor](https://rust-lang.github.io/async-book/02_execution/04_executor.html)：看任务、唤醒、再次轮询的整体关系；它的多任务实现和依赖暂不照搬。
5. [Rust Async Book：Executors and System IO](https://rust-lang.github.io/async-book/02_execution/05_io.html)：完成阶段 D 后选读真实 I/O 驱动如何接入。

## 9. 下一主题

完成本主题后，再研究真实非阻塞 I/O 与 Tokio：对照 Day 2 的 `TcpStream::read`，说明异步 socket 在尚无数据时如何返回等待状态、由谁登记 I/O 就绪、谁负责唤醒任务。不要在尚不能解释 `Pending → wake → 再次 poll` 时直接重写 Echo 服务器。
