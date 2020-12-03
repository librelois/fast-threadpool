//! This threadpool implementation is optimized to minimize latency.
//! In particular, you are guaranteed never to pay the cost of thread spawn before your task is executed.
//! New threads are spawned only during the "useless time" of the workers (for example, after returning the result of a job).
//!
//! The only possible case of latency is the lack of "available" workers.
//! To minimize the probability of this case, this threadpool constantly keeps a certain number of workers available (configurable).
//!
//! This implementation allows you to wait for the result of a job asynchronously,
//! so you can use it as a replacement for the `spawn_blocking` function of your async runtime.
//!
//! ## Use
//! ```rust
//! use fast_threadpool::{ThreadPool, ThreadPoolConfig};
//!
//! let threadpool = ThreadPool::start(ThreadPoolConfig::default(), ()).into_sync_handler();
//!
//! assert_eq!(4, threadpool.execute(|_| { 2 + 2 })?);
//! # Ok::<(), fast_threadpool::ThreadPoolDisconnected>(())
//! ```
//!
//! ## Use from async task
//!
//! ```rust
//! # futures::executor::block_on(async {
//! use fast_threadpool::{ThreadPool, ThreadPoolConfig};
//
//! let threadpool = ThreadPool::start(ThreadPoolConfig::default(), ()).into_async_handler();
//!
//! assert_eq!(4, threadpool.execute(|_| { 2 + 2 }).await?);
//! # Ok::<(), fast_threadpool::ThreadPoolDisconnected>(())
//! # });
//! ```

#![cfg_attr(test, recursion_limit = "256")]
#![deny(
    clippy::unwrap_used,
    missing_docs,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unstable_features,
    unused_import_braces,
    unused_qualifications
)]

mod config;
mod handler;
mod state;
mod worker;

pub use crate::config::ThreadPoolConfig;
pub use crate::handler::{JoinHandle, ThreadPoolAsyncHandler, ThreadPoolSyncHandler};

use crate::state::State;
use crate::worker::{MsgForWorker, Worker};
use flume::{Receiver as FlumeReceiver, RecvTimeoutError, Sender as FlumeSender};
use std::{
    num::NonZeroU16,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::Duration,
};

#[derive(Clone, Copy, Debug)]
/// Thread pool disconnected
pub struct ThreadPoolDisconnected;

impl std::fmt::Display for ThreadPoolDisconnected {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Thread pool disconnected")
    }
}
impl std::error::Error for ThreadPoolDisconnected {}

#[derive(Debug)]
/// A fast thread pool (never pay the cost of thread spawn.)
///
pub struct ThreadPool<Shared: 'static + Clone + Send> {
    sender: FlumeSender<MsgForWorker<Shared>>,
}

impl<Shared: 'static + Clone + Send> ThreadPool<Shared> {
    /// Start a thread pool
    /// This function spawn `min_workers` threads.
    pub fn start(config: ThreadPoolConfig, shared: Shared) -> Self {
        let state = State::new(config);
        let (sender, receiver) = if let Some(queue_size) = config.queue_size {
            flume::bounded(queue_size)
        } else {
            flume::unbounded()
        };

        for _ in 0..config.min_workers.get() {
            let worker = Worker::new(
                config.keep_alive,
                receiver.clone(),
                sender.clone(),
                shared.clone(),
                state.clone(),
            );
            std::thread::spawn(move || worker.run());
        }

        ThreadPool { sender }
    }
    /// Get an asynchronous handler
    pub fn async_handler(&self) -> ThreadPoolAsyncHandler<Shared> {
        ThreadPoolAsyncHandler::new(self.sender.clone())
    }
    /// Get a synchronous handler
    pub fn sync_handler(&self) -> ThreadPoolSyncHandler<Shared> {
        ThreadPoolSyncHandler::new(self.sender.clone())
    }
    /// Convert threadpool into an asynchronous handler
    pub fn into_async_handler(self) -> ThreadPoolAsyncHandler<Shared> {
        ThreadPoolAsyncHandler::new(self.sender)
    }
    /// Convert threadpool into a synchronous handler
    pub fn into_sync_handler(self) -> ThreadPoolSyncHandler<Shared> {
        ThreadPoolSyncHandler::new(self.sender)
    }
}

type Job<Shared> = Box<dyn FnOnce(&Shared) + Send + 'static>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    const FOUR: NonZeroU16 = unsafe { NonZeroU16::new_unchecked(4) };

    #[test]
    fn test_sync() -> Result<(), ThreadPoolDisconnected> {
        let tp = ThreadPool::start(ThreadPoolConfig::default(), ());

        let tp_handler = tp.into_sync_handler();

        assert_eq!(4, tp_handler.execute(|_| { 2 + 2 })?);

        let start = Instant::now();

        let r1 = tp_handler.launch(|_| std::thread::sleep(Duration::from_secs(1)))?;
        let r2 = tp_handler.launch(|_| std::thread::sleep(Duration::from_secs(1)))?;
        let r3 = tp_handler.launch(|_| std::thread::sleep(Duration::from_secs(1)))?;
        let r4 = tp_handler.launch(|_| std::thread::sleep(Duration::from_secs(1)))?;

        r1.join().expect("ThreadPool disconnected");
        r2.join().expect("ThreadPool disconnected");
        r3.join().expect("ThreadPool disconnected");
        r4.join().expect("ThreadPool disconnected");

        let elapsed = start.elapsed();

        assert!(elapsed.as_secs() < 2);

        Ok(())
    }

    #[test]
    fn test_async() -> Result<(), ThreadPoolDisconnected> {
        futures::executor::block_on(async {
            let shared: i32 = 42;
            let conf = ThreadPoolConfig::default()
                .min_workers(FOUR)
                .max_available_workers(FOUR);
            println!("conf={:?}", conf);
            let tp = ThreadPool::start(conf, shared);

            let tp_handler = tp.into_async_handler();

            assert_eq!(4u32, tp_handler.execute(|_| { 2 + 2 }).await?);

            let start = Instant::now();

            use futures::join;
            let (res1, res2, res3, res4, res5, res6, res7, res8, res9, res10) = join!(
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
                tp_handler.execute(|_| { std::thread::sleep(Duration::from_secs(1)) }),
            );

            res1?;
            res2?;
            res3?;
            res4?;
            res5?;
            res6?;
            res7?;
            res8?;
            res9?;
            res10?;

            let elapsed = start.elapsed();

            assert!(elapsed.as_secs() < 3);

            Ok(())
        })
    }
}
