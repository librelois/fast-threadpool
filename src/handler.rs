use crate::*;

#[derive(Clone, Debug)]
/// Asynchronous handler to execute jobs on the thread pool
pub struct ThreadPoolAsyncHandler<Shared: 'static + Clone + Send> {
    sender: Sender<MsgForWorker<Shared>>,
}

impl<Shared: 'static + Clone + Send> ThreadPoolAsyncHandler<Shared> {
    pub(crate) fn new(sender: Sender<MsgForWorker<Shared>>) -> ThreadPoolAsyncHandler<Shared> {
        ThreadPoolAsyncHandler { sender }
    }
    /// Execute the given closure and return a Future that output closure return type
    pub async fn execute<F, R>(&self, f: F) -> Result<R, ThreadPoolDisconnected>
    where
        F: 'static + Send + FnOnce(&Shared) -> R,
        R: 'static + Send,
    {
        let (s, r) = oneshot::channel();
        self.sender
            .send_async(MsgForWorker::NewJob(Box::new(move |shared| {
                let _ = s.send(f(shared));
            })))
            .await
            .map_err(|_| ThreadPoolDisconnected)?;

        r.await.map_err(|_| ThreadPoolDisconnected)
    }
}

#[derive(Clone, Debug)]
/// Synchronous handler to execute jobs on the thread pool
pub struct ThreadPoolSyncHandler<Shared: 'static + Clone + Send> {
    sender: Sender<MsgForWorker<Shared>>,
}

impl<Shared: 'static + Clone + Send> ThreadPoolSyncHandler<Shared> {
    pub(crate) fn new(sender: Sender<MsgForWorker<Shared>>) -> ThreadPoolSyncHandler<Shared> {
        ThreadPoolSyncHandler { sender }
    }
    /// Execute the given closure and block the current thread until finished
    /// If you need a non blocking method, see `ThreadPoolAsyncHandler`.
    pub fn execute<F, R>(&self, f: F) -> Result<R, ThreadPoolDisconnected>
    where
        F: 'static + Send + FnOnce(&Shared) -> R,
        R: 'static + Send,
    {
        let (s, r) = oneshot::channel();
        self.sender
            .send(MsgForWorker::NewJob(Box::new(move |shared| {
                let _ = s.send(f(shared));
            })))
            .map_err(|_| ThreadPoolDisconnected)?;

        r.recv().map_err(|_| ThreadPoolDisconnected)
    }
}
