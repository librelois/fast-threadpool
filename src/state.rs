use crate::*;

#[derive(Clone, Debug)]
pub(crate) struct State {
    inner: Arc<AtomicU32>,
    max_available_workers: u32,
    max_workers: u32,
    min_available_workers: u32,
    min_workers: u32,
}

const AVAILABLE_MASK: u32 = 0b_0000_0000_0000_0000_1111_1111_1111_1111;
const WORKER_COUNT_BASE: u32 = 0b_0001_0000_0000_0000_0000;

impl State {
    pub(crate) fn new(conf: ThreadPoolConfig) -> Self {
        Self {
            inner: Arc::new(AtomicU32::new(0)),
            max_available_workers: conf.max_available_workers.get() as u32,
            max_workers: conf.max_workers.get() as u32,
            min_available_workers: conf.min_available_workers.get() as u32,
            min_workers: conf.min_workers.get() as u32,
        }
    }
    pub(crate) fn decrease_available(&self) -> bool {
        let previous_state = self.inner.fetch_sub(1, Ordering::SeqCst);
        let available_count = previous_state & AVAILABLE_MASK;
        //println!("TMP DEBUG: available_count={}", available_count);
        (available_count - 1) <= self.max_available_workers
    }
    pub(crate) fn increment_available(&self) -> bool {
        let previous_state = self.inner.fetch_add(1, Ordering::SeqCst);
        let available_count = previous_state & AVAILABLE_MASK;
        let worker_count = previous_state >> 16;
        (available_count + 1) <= self.max_available_workers && worker_count < self.max_workers
    }
    pub(crate) fn allow_new_worker(&self) -> bool {
        self.inner
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |state| {
                let worker_count = state >> 16;
                if worker_count < self.max_workers {
                    Some(state + WORKER_COUNT_BASE + 1)
                } else {
                    None
                }
            })
            .is_ok()
    }
    pub(crate) fn need_spawn(&self) -> bool {
        let state = self.inner.load(Ordering::SeqCst);
        let available_count = state & AVAILABLE_MASK;
        let worker_count = state >> 16;
        available_count <= self.max_available_workers && worker_count < self.max_workers
    }
    pub(crate) fn allow_to_shutdown(&self) -> bool {
        self.inner
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |state| {
                let available_count = state & AVAILABLE_MASK;
                let worker_count = state >> 16;
                if available_count > self.min_available_workers && worker_count > self.min_workers {
                    Some(state - WORKER_COUNT_BASE)
                } else {
                    None
                }
            })
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EIGHT: NonZeroU16 = unsafe { NonZeroU16::new_unchecked(8) };

    #[test]
    fn test_state() {
        let state = State::new(
            ThreadPoolConfig::default()
                .min_workers(EIGHT)
                .max_available_workers(EIGHT),
        );

        for _ in 0..8 {
            assert!(state.allow_new_worker());
        }

        for _ in 0..8 {
            assert!(state.decrease_available());
        }
    }
}
