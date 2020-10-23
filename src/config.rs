use crate::*;

#[derive(Clone, Copy, Debug)]
/// Thread pool configuration
pub struct ThreadPoolConfig {
    pub(crate) keep_alive: Duration,
    pub(crate) min_workers: NonZeroU16,
    pub(crate) max_workers: NonZeroU16,
    pub(crate) min_available_workers: NonZeroU16,
    pub(crate) max_available_workers: NonZeroU16,
    pub(crate) queue_size: Option<usize>,
}

impl Default for ThreadPoolConfig {
    fn default() -> Self {
        let min_workers = NonZeroU16::new(num_cpus::get() as u16)
            .unwrap_or(unsafe { NonZeroU16::new_unchecked(4) });
        ThreadPoolConfig {
            keep_alive: Duration::from_secs(30),
            min_workers,
            max_workers: unsafe { NonZeroU16::new_unchecked(512) },
            min_available_workers: unsafe { NonZeroU16::new_unchecked(4) },
            max_available_workers: min_workers,
            queue_size: Some(64),
        }
    }
}

impl ThreadPoolConfig {
    ///
    pub fn keep_alive(mut self, keep_alive: u64) -> Self {
        self.keep_alive = Duration::from_secs(keep_alive);
        self
    }
    ///
    pub fn min_workers(mut self, min_workers: NonZeroU16) -> Self {
        self.min_workers = min_workers;
        self
    }
    ///
    pub fn max_workers(mut self, max_workers: NonZeroU16) -> Self {
        assert!(self.max_available_workers < max_workers);
        self.max_workers = max_workers;
        self
    }
    ///
    pub fn min_available_workers(mut self, min_available_workers: NonZeroU16) -> Self {
        self.min_available_workers = min_available_workers;
        self
    }
    ///
    pub fn max_available_workers(mut self, max_available_workers: NonZeroU16) -> Self {
        assert!(max_available_workers < self.max_workers);
        self.max_available_workers = max_available_workers;
        self
    }
    ///
    pub fn queue_size(mut self, queue_size: Option<usize>) -> Self {
        self.queue_size = queue_size;
        self
    }
}
