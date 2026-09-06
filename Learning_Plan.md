# Rust Career Learning Plan

## 1. Background

I am a first-year master's student currently conducting research related to Rust.

I have approximately **one year to prepare for Rust-related employment**.

My current Rust foundation:

- I have completed *The Rust Programming Language* (the official Rust Book).
- I understand the core Rust language concepts, including:
  - Ownership and borrowing
  - Lifetimes
  - Traits and generics
  - Enums and pattern matching
  - `Box`, `Rc`, `Arc`
  - `RefCell`, interior mutability
  - Basic concurrency concepts
  - Basic `Future` / `async` / `await` concepts
- However, most of my experience is still at the level of **learning language concepts**.
- I currently lack substantial engineering experience building real Rust systems.

Therefore, the focus of this repository is **not to relearn Rust syntax**, but to develop practical Rust engineering skills through progressively more realistic projects.

---

# 2. Long-Term Goal

My goal is to become employable for Rust-related engineering positions within approximately one year.

My target position is:

> **Rust backend / infrastructure engineer, with a focus on high concurrency,
> databases, and distributed systems, followed by AI inference and compute
> infrastructure.**

The learning order is deliberate:

1. Become a capable production backend engineer.
2. Develop systems depth in networking, storage, distributed systems, and performance.
3. Apply those skills to AI inference and compute infrastructure.

Embedded Rust is no longer a primary track for this plan. It may be revisited
later, but it should not compete with backend and infrastructure preparation
during the current one-year employment window.

Both directions require strong foundations in:

- Systems programming
- Network programming
- Concurrency
- Asynchronous programming
- Performance engineering
- Linux
- Distributed systems fundamentals

Therefore, the first phase of this learning plan focuses heavily on:

> **Rust networking + concurrency + async programming**

The goal is not simply to know how to use Tokio APIs.

I want to understand the progression:

```text
OS / TCP / blocking I/O
        ↓
Threads
        ↓
Concurrency
        ↓
Future
        ↓
async / await
        ↓
Runtime
        ↓
Tokio
        ↓
Async network services
        ↓
Backend / distributed systems / AI Infra
```

---

# 3. One-Year Target

After approximately one year, I want to be capable of independently building and understanding non-trivial Rust systems.

Employment readiness means more than knowing Rust syntax. I should have
evidence that I can design, implement, test, deploy, observe, debug, and explain
a production-oriented service.

I should be comfortable with the following areas.

## Rust

- Ownership / borrowing / lifetimes
- Traits and generics
- Error handling
- Smart pointers
- Interior mutability
- `Send` / `Sync`
- Threads
- Atomics
- Channels
- `Arc` / `Mutex` / `RwLock`
- Futures
- async / await
- Pinning
- Basic unsafe Rust
- Cargo workspace and project organization

## Networking

- TCP / UDP fundamentals
- Socket programming
- Blocking vs non-blocking I/O
- TCP byte-stream semantics
- Application-level protocols
- HTTP fundamentals
- Connection management
- Basic network debugging

## Async Rust

I should understand rather than merely use:

- `Future`
- `poll`
- `Poll::Ready`
- `Poll::Pending`
- `Waker`
- async state machines
- executors
- reactors
- Tokio runtime
- async I/O
- task scheduling
- cancellation
- channels
- synchronization

## Backend Engineering

Eventually I should be able to work with:

- Tokio
- Axum or similar frameworks
- HTTP APIs
- SQL databases
- Redis
- serialization
- logging / tracing
- configuration
- testing
- observability
- graceful shutdown
- production-oriented project organization

## Systems / AI Infrastructure

Later stages should introduce:

- Linux systems programming
- Memory and performance analysis
- Profiling
- Zero-copy concepts
- High-performance networking
- RPC
- gRPC
- Distributed systems fundamentals
- Storage systems
- Caching
- Queues
- Scheduling
- Rust/Python interoperability
- AI serving / inference infrastructure concepts

---

# 4. Learning Philosophy

This repository is primarily a **coding training repository**.

The learning ratio should roughly be:

```text
30% reading / theory
70% coding / experiments / debugging
```

Avoid spending large amounts of time passively reading tutorials.

The preferred learning cycle is:

```text
Learn one concept
        ↓
Predict program behavior
        ↓
Write a small experiment
        ↓
Run it
        ↓
Observe the result
        ↓
Explain why
        ↓
Modify the program
        ↓
Encounter limitations
        ↓
Learn the next concept
```

Projects should evolve naturally from limitations in previous implementations.

For example:

```text
Blocking TCP Server
        ↓
"Why can't it handle clients concurrently?"
        ↓
Thread-per-connection Server
        ↓
"What happens with thousands of connections?"
        ↓
Non-blocking / async concepts
        ↓
Future
        ↓
Tokio
```

I prefer learning this way instead of learning APIs independently without understanding why they exist.

---

# 5. How Codex Should Help Me

Codex should behave primarily as a:

> **Rust mentor + code reviewer + learning guide**

rather than an implementation agent.

The purpose is to improve my ability to write Rust independently.

## Important Rule: Do Not Immediately Give Me Complete Solutions

When I am doing a learning exercise, Codex should normally NOT immediately generate the complete implementation.

Prefer this process:

1. Explain the relevant concept.
2. Give me a small, concrete task.
3. Let me implement it.
4. Review my implementation.
5. Point out bugs, design problems, and Rust-specific issues.
6. Ask me to reason about important behavior.
7. Give hints when necessary.
8. Only provide a complete implementation when:
   - I explicitly request it, or
   - I am genuinely stuck after attempting the task.

For example, instead of immediately implementing a TCP Echo Server, guide me through:

```text
TcpListener::bind
        ↓
accept
        ↓
TcpStream
        ↓
Read
        ↓
Write
        ↓
loop
        ↓
connection close
```

---

# 6. Code Review Expectations

When I submit code, review it from several perspectives.

## Correctness

Check for:

- Incorrect assumptions
- Edge cases
- Resource handling
- Network behavior
- Error handling

## Rust Semantics

Pay particular attention to:

- Ownership
- Borrowing
- Lifetimes
- Moves
- `Send` / `Sync`
- Smart pointer usage
- Error propagation

## Engineering Quality

Consider:

- API design
- Function boundaries
- Naming
- Error handling
- Maintainability
- Testability
- Project organization

## Performance

When relevant, discuss:

- Allocations
- Copies
- Blocking
- Lock contention
- Task/thread overhead
- Memory usage

However, do not prematurely optimize simple learning exercises.

---

# 7. Ask Me to Predict Behavior

An important part of the learning process is reasoning before execution.

Codex should frequently ask questions such as:

- Will this call block?
- Who owns this value?
- What happens when this variable is dropped?
- What does this `read()` return?
- What happens when the client disconnects?
- Can two threads access this value?
- Is this type `Send`?
- Why does the compiler reject this?
- What happens if 1,000 clients connect?
- Where is the task suspended at `.await`?

I should attempt to answer before Codex explains the result.

---

# 8. Repository Structure

This repository uses a Cargo Workspace.

The approximate structure is:

```text
.
├── Cargo.toml
├── README.md
├── LEARNING_PLAN.md
│
├── .devcontainer/
├── .vscode/
│
├── week1/
│   ├── day1_tcp_echo/
│   ├── day2_thread_server/
│   ├── day3_future/
│   ├── day4_tokio_echo/
│   └── day5_chat_server/
│
└── projects/
```

Each learning exercise should generally be a small independent crate.

The repository should gradually become both:

1. A learning record
2. A Rust engineering portfolio

Therefore, code quality should improve over time.

---

# 9. Current Phase

I am currently in:

> **Phase 1 — Rust Networking, Concurrency, and Async Foundations**

The immediate objective is:

> Progress from a blocking TCP server to an asynchronous Tokio-based concurrent network service while understanding each abstraction introduced along the way.

---

# 10. Employment-Oriented Roadmap

The detailed roadmap is maintained in:

```text
docs/career_roadmap.md
```

The main progression is:

```text
Networking / concurrency / async foundations
        ->
Production Rust backend engineering
        ->
Databases / storage / distributed systems / performance
        ->
AI inference and compute infrastructure
```

Three portfolio projects should evolve throughout the year instead of being
discarded after short exercises:

1. A production-oriented backend service using Axum, PostgreSQL, Redis,
   testing, observability, Docker, and CI.
2. A networked key-value system that gradually gains protocol framing,
   persistence, concurrency control, performance testing, and selected
   distributed-system features.
3. An AI inference gateway or scheduler connecting a Rust service to Python
   model workers, with batching, backpressure, metrics, and failure handling.

Data structures and algorithms, Linux, computer networks, databases, English
technical reading, system design, and job tracking are continuous parallel
tracks throughout the year.

The schedule is milestone-based. A "day" in an exercise plan is a learning
unit and may take more than one calendar day. Move forward only when the
completion evidence is met.

Before each learning unit, prepare both a teaching plan and a Chinese learning
document. The learning document should include a small set of relevant Chinese
or English references, label required versus optional reading, and explain what
to focus on. Prefer official documentation and high-quality primary material.
Reading supports the current experiment and must not replace implementation,
prediction, observation, and explanation.

---

# 11. Week 1 Plan

The first week focuses on networking, concurrency, async foundations, and
protocol correctness. Chat and Mini KV are moved to later project phases so
the foundations are not rushed.

## Day 1 — Blocking TCP

Topics:

- `TcpListener`
- `TcpStream`
- sockets
- `bind`
- `accept`
- `Read`
- `Write`
- blocking I/O
- TCP byte streams
- connection termination

Target:

Build a TCP Echo Server using only the Rust standard library.

No Tokio.

---

## Day 2 — Threads and Concurrency

Upgrade the blocking server to support multiple clients.

Topics:

- `std::thread`
- `thread::spawn`
- ownership across threads
- `move`
- `Send`
- `Arc`
- `Mutex`
- thread-per-connection model

Important question:

> Why doesn't a simple blocking server handle multiple clients well?

Then:

> Why isn't creating one OS thread for every connection an ideal solution at large scale?

---

## Day 3 — Future and Async Fundamentals

Understand why async Rust exists.

Topics:

- blocking vs non-blocking
- cooperative execution
- `Future`
- `poll`
- `Poll`
- `Ready`
- `Pending`
- `Context`
- `Waker`
- basic executor concepts

The goal is not to memorize the `Future` trait.

The goal is to understand conceptually:

```text
Future
    ↓
poll()
    ↓
Ready(value)

or

Pending
    ↓
wake me later
```

---

## Day 4 — Tokio

Rewrite the TCP server using Tokio.

Topics:

- Tokio runtime
- async main
- async `TcpListener`
- async `TcpStream`
- `.await`
- `tokio::spawn`
- tasks vs threads

Compare directly with the Day 1 blocking implementation.

---

## Day 5 — Async Connection Lifecycle

Make the Tokio Echo Server robust under multiple clients.

Topics:

- multiple connections
- Tokio tasks
- connection lifecycle
- task ownership
- cancellation
- timeouts
- graceful shutdown
- basic error propagation

---

## Day 6 — Protocol Framing and Tests

Turn the byte-stream experiment into a small framed protocol and test its
boundaries.

Topics:

- protocol parsing
- TCP packet/message boundary mismatch
- newline or length-prefix framing
- partial reads and multiple frames in one read
- unit tests for parsing
- integration tests for network behavior

---

## Day 7 — Review and Refactoring

Review everything from the week.

Tasks:

- Refactor code
- Improve error handling
- Add README documentation
- Compare blocking/thread/async architectures
- Explain important concepts without looking at notes
- Identify weak areas
- Plan Week 2 based on actual progress

---

# 12. Current Progress

Current position:

```text
Week 1
└── Day 2 — Thread-per-connection server
```

Completed concepts:

### `TcpListener`

I understand that:

```rust
TcpListener::bind("127.0.0.1:8080")
```

creates/binds a server-side listening socket.

`TcpListener` is responsible for accepting new TCP connections.

Multiple clients may connect to the same server listening port.

---

### `accept()`

I understand:

```rust
let (stream, addr) = listener.accept().unwrap();
```

When there is no pending client connection:

```text
accept()
   ↓
current thread blocks
   ↓
OS waits for a connection
   ↓
connection arrives
   ↓
thread continues
```

`accept()` returns:

- `TcpStream`
- `SocketAddr`

`TcpStream` represents the local Rust handle/abstraction for one established TCP connection.

`SocketAddr` identifies the peer using its IP address and port.

---

### TCP Connections

I understand that clients can all connect to the same server port while still having independent TCP connections.

Conceptually:

```text
Client A :51001 ────── Server :8080
Client B :51002 ────── Server :8080
Client C :51003 ────── Server :8080
```

A TCP connection can be identified by the source/destination IP and port tuple.

---

### Blocking `read()`

I understand that:

```rust
stream.read(&mut buffer)
```

also blocks when the connection is established but no data is currently available.

Therefore, I currently understand two important blocking points:

```text
accept()
   ↓
wait for connection

read()
   ↓
wait for data
```

---

### `Read::read`

Given:

```rust
let mut buffer = [0u8; 1024];

let n = stream.read(&mut buffer).unwrap();
```

I understand that `n` is the number of bytes actually read.

Therefore, valid data is generally:

```rust
&buffer[..n]
```

rather than the entire 1024-byte buffer.

---

### TCP Is a Byte Stream

I have been introduced to the fact that TCP is a byte-stream protocol.

There is NOT necessarily a one-to-one relationship:

```text
send() → read()
```

For example:

```text
Client:

send("hello")
send("world")
```

does not guarantee:

```text
Server:

read() → "hello"
read() → "world"
```

The server may observe different byte groupings.

This concept still needs further practical experimentation.

---

### `Write::write_all` and TCP shutdown

I implemented echoing only the valid bytes with `&buffer[..n]` and verified it
with a PowerShell TCP client.

I understand that `write_all` first hands all bytes to the local operating
system. The TCP stack delivers them reliably and in order; a client can still
read already delivered data before observing EOF after the peer closes.

---

### `read() == 0`

I observed that an orderly client disconnect makes the server's next `read()`
return `Ok(0)`. I handle this case by leaving the connection loop, avoiding a
busy loop that repeatedly reads zero bytes.

---

### Single-threaded connection limitation

I implemented an outer accept loop and an inner read/echo loop. The server can
serve clients sequentially, but I verified that it cannot actively process
client B while it is blocked reading from client A.

This observed limitation is the motivation for Day 2.

---

# 13. Current Exercise

I am starting Day 2: moving each accepted connection to its own OS thread.

The current task is to create `week1/day2_thread_server` and first extract the
Day 1 per-connection behavior into a function that owns its `TcpStream`:

```text
main thread
  accept()
     ↓
owned TcpStream
     ↓
handle_connection(stream, addr)
```

After the handler works without threads, the next experiment should attempt to
use it with `thread::spawn`, inspect the ownership or lifetime error, and then
reason about `move`, `Send`, and `'static` before fixing it.

---

# 14. Important Instruction for Continuing From Here

When continuing this learning session, do NOT jump directly to Tokio or provide
the complete thread-per-connection implementation before I attempt it.

Continue from the Day 2 handler extraction.

The intended progression is:

```text
extract connection handler
  ↓
attempt thread::spawn
  ↓
understand the compiler error
  ↓
closure capture and move
  ↓
ownership transfer
  ↓
Send and 'static
  ↓
verify concurrent clients
  ↓
identify thread-per-connection costs
```

At each important step:

> Ask me to predict what will happen before explaining the answer.

The goal is not to finish exercises as quickly as possible.

The goal is for me to eventually be able to design, implement, debug, and explain Rust systems independently.
