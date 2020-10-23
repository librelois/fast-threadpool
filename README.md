# Fast Thread-pool

This thread-pool implementation is optimized to minimize latency.
In particular, you are guaranteed to never pay the cost of thread spawn before your task is executed.
New threads are spawned only during the "useless time" of the workers (for example, after returning the result of a job).

The only possible case of latency is the lack of "available" workers.
To minimize the probability of this case, this thread-pool constantly keeps a certain number of workers available (configurable).

This implementation allows you to wait for the result of a job asynchronously,
so you can use it as a replacement for the `spawn_blocking` function of your async runtime.

## Use

```rust
use fast_threadpool::{ThreadPool, ThreadPoolConfig};

let threadpool = ThreadPool::start(ThreadPoolConfig::default(), ()).into_sync_handler();

assert_eq!(4, threadpool.execute(|_| { 2 + 2 })?);
```

## Use from async task

```rust
use fast_threadpool::{ThreadPool, ThreadPoolConfig};

let threadpool = ThreadPool::start(ThreadPoolConfig::default(), ()).into_async_handler();

assert_eq!(4, threadpool.execute(|_| { 2 + 2 }).await?);
```
