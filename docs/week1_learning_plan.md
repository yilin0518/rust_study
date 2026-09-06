# Week 1: Networking, Concurrency, and Async Foundations

## Week outcome

Build and explain the evolution from a single-client blocking TCP server to a
robust asynchronous multi-client protocol service. The goal is to understand
why each abstraction is needed and establish the networking foundation for
production backend engineering.

## Working rhythm for every day

1. Spend 10–15 minutes recalling yesterday's model without reading notes.
2. Predict the behavior of the next experiment before running it.
3. Implement one small change independently.
4. Run it, record the observed behavior, and explain it in your own words.
5. End with a short checkpoint: what problem remains unsolved?

Keep each day's code in its own workspace crate. Use the standard library until
Day 4, when Tokio is introduced deliberately.

---

## Day 1 — Blocking TCP Echo Server

**Outcome:** A server that accepts one client and echoes every received byte
until that client disconnects.

**Concepts**

- `TcpListener::bind`, `accept`, and `TcpStream`
- `Read::read` and `Write::write_all`
- blocking points: `accept()` and `read()`
- `read() == 0` as orderly peer shutdown
- TCP as a byte stream, not a message protocol

**Exercises**

1. Complete the existing read experiment and print only `&buffer[..n]`.
2. Predict what a second `read()` does when the client is still connected but
   sends no bytes; run the experiment.
3. Predict what `read()` returns when the client closes its sending side or
   disconnects; run it.
4. Turn the single read into a read loop. On each nonzero read, echo exactly
   the valid slice back to the client.
5. Connect two clients. Observe why the second one cannot be served while the
   first connection is being read.

**Checkpoint questions**

- Why must the server use `n` rather than the buffer length?
- Does one client `send` correspond to one server `read`?
- Which call prevents the server from accepting another client?

**Deliverable:** `week1/day1_tcp_echo`, plus a short note describing the two
blocking points and the meaning of zero bytes read.

---

## Day 2 — Thread per Connection

**Outcome:** A blocking server that serves several clients concurrently using
one OS thread per accepted connection.

**Concepts**

- `thread::spawn`, closures, and `move`
- ownership transfer to a thread
- `Send` and the `'static` requirement
- `TcpStream` ownership per connection
- the cost and limitations of thread-per-connection

**Exercises**

1. Extract Day 1's per-connection loop into a handler function.
2. Predict why borrowing `stream` inside `thread::spawn` is rejected.
3. Move the owned stream into the spawned closure and test with two clients.
4. Add connection lifecycle logs containing the peer address.
5. Make several clients wait or send slowly. Explain why this design works,
   and why thousands of threads would become expensive.

**Checkpoint questions**

- Who owns `TcpStream` after it is moved into the spawned closure?
- What does `Send` permit here?
- What bottleneck did threads remove, and what costs did they add?

**Deliverable:** `week1/day2_thread_server` and a comparison with Day 1.

---

## Day 3 — Futures and Cooperative Async

**Outcome:** Explain the `Future` polling model and build a tiny manual future
experiment; no network framework yet.

**Concepts**

- blocking versus non-blocking work
- `Future`, `poll`, `Poll::Ready`, and `Poll::Pending`
- `Context` and `Waker`
- cooperative scheduling and the executor's role
- why `.await` is a suspension point rather than a new thread

**Exercises**

1. Trace a simple future whose first poll returns `Pending` and a later poll
   returns `Ready`.
2. Before seeing the result, predict what an executor should do after
   `Pending`.
3. Implement or inspect one small custom future state machine.
4. Draw the relationship among future, executor, waker, and event source.
5. Compare an idle blocking thread with a pending async task.

**Checkpoint questions**

- Why must a future avoid blocking inside `poll`?
- Who decides when a pending future is polled again?
- What problem does a waker solve?

**Deliverable:** `week1/day3_future` and a one-page explanation in your own
words.

---

## Day 4 — Tokio Echo Server

**Outcome:** Rewrite the echo server with Tokio and relate every new API to
the Day 1 and Day 2 designs.

**Concepts**

- Tokio runtime and `#[tokio::main]`
- asynchronous listener and stream I/O
- `.await`
- `tokio::spawn`
- tasks versus OS threads

**Exercises**

1. Add Tokio only after stating what limitation it addresses.
2. Rewrite listener setup and per-connection reads with Tokio APIs.
3. Spawn one task per connection and test concurrent clients.
4. Identify the points where the task yields instead of blocking its thread.
5. Compare the ownership model of a Tokio task with Day 2's thread closure.

**Checkpoint questions**

- What wakes a task waiting for socket data?
- Why can many tasks share relatively few OS threads?
- Which TCP byte-stream rules remain unchanged under Tokio?

**Deliverable:** `week1/day4_tokio_echo` and a direct Day 2 comparison.

---

## Day 5 — Async Connection Lifecycle

**Outcome:** A Tokio server that handles multiple connection lifecycles and
fails predictably under timeout, disconnect, and shutdown conditions.

**Concepts**

- connection lifecycle
- Tokio tasks
- timeouts and cancellation
- error propagation
- graceful shutdown
- basic backpressure awareness

**Exercises**

1. Handle read, write, and disconnect errors without panicking globally.
2. Add and observe an idle-connection timeout.
3. Identify cancellation points around `.await`.
4. Stop accepting new connections during shutdown.
5. Allow active connection tasks to finish or cancel deliberately.

**Checkpoint questions**

- What is cancelled when a task is dropped?
- Which errors belong to one connection and which stop the whole server?
- What should happen to active tasks during shutdown?

**Deliverable:** a robust revision of `week1/day4_tokio_echo` with documented
connection lifecycle behavior.

---

## Day 6 — Protocol Framing and Tests

**Outcome:** A small framed protocol that behaves correctly when one message is
split across reads or several messages arrive in one read.

**Concepts**

- TCP byte-stream framing
- newline and length-prefix framing trade-offs
- buffering partial frames
- parsing and recoverable invalid input
- unit and network integration tests

**Exercises**

1. Write protocol examples and boundary cases before parsing.
2. Force one logical message to arrive through several writes.
3. Send several logical messages through one write.
4. Implement buffering that extracts complete frames and retains the remainder.
5. Test normal, partial, combined, oversized, and invalid frames.

**Checkpoint questions**

- Why can neither `send()` nor `read()` define a message boundary?
- Who owns bytes that belong to an incomplete frame?
- What limit prevents an attacker from growing the buffer forever?

**Deliverable:** `week1/day6_framed_protocol` with parser tests and protocol
documentation.

---

## Day 7 — Consolidation and Review

**Outcome:** Demonstrate understanding of the week's tradeoffs and leave the
workspace in a documented, runnable state.

**Exercises**

1. Run formatting, checks, and the relevant tests or manual verification.
2. Improve only error handling and structure that obscures the learning goal.
3. Write a short architecture comparison: blocking single client,
   thread-per-connection, and Tokio tasks.
4. Explain from memory: `read() == 0`, TCP byte streams, `move`, `Send`,
   `Poll::Pending`, wakers, and task versus thread.
5. Record questions or weak points to steer Week 2.

**Completion evidence**

- Each crate builds.
- You can manually demonstrate the behavior of its network server.
- You can explain the limitation that motivated the next day's abstraction.

---

## Today: resume point

Start at **Day 2** after completing the blocking TCP exercises. Extract the
Day 1 connection loop into a handler, then investigate ownership transfer into
`thread::spawn` before using Tokio.
