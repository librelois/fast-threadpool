use crate::*;

pub(crate) enum MsgForWorker<Shared: Clone + Send> {
    NewJob(Job<Shared>),
    Spawn,
}

#[derive(Clone)]
pub(crate) struct Worker<Shared: 'static + Clone + Send> {
    keep_alive: Duration,
    receiver: Receiver<MsgForWorker<Shared>>,
    sender: Sender<MsgForWorker<Shared>>,
    shared: Shared,
    state: State,
}

impl<Shared: 'static + Clone + Send> Worker<Shared> {
    pub(crate) fn new(
        keep_alive: Duration,
        receiver: Receiver<MsgForWorker<Shared>>,
        sender: Sender<MsgForWorker<Shared>>,
        shared: Shared,
        state: State,
    ) -> Self {
        Worker {
            keep_alive,
            receiver,
            sender,
            shared,
            state,
        }
    }
    pub(crate) fn run(self) {
        if self.state.allow_new_worker() {
            loop {
                match self.receiver.recv_timeout(self.keep_alive) {
                    Ok(msg) => match msg {
                        MsgForWorker::NewJob(job) => {
                            if self.state.decrease_available() {
                                let _ = self.sender.try_send(MsgForWorker::Spawn);
                            }
                            job(&self.shared);
                            if self.state.increment_available() {
                                let new_worker = self.clone();
                                std::thread::spawn(move || new_worker.run());
                            }
                        }
                        MsgForWorker::Spawn => {
                            if self.state.need_spawn() {
                                let new_worker = self.clone();
                                std::thread::spawn(move || new_worker.run());
                            }
                        }
                    },
                    Err(RecvTimeoutError::Timeout) => {
                        if self.state.need_spawn() {
                            let new_worker = self.clone();
                            std::thread::spawn(move || new_worker.run());
                        }
                        if self.state.allow_to_shutdown() {
                            break;
                        } else {
                            continue;
                        }
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        }
    }
}
