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
        R: 'static + Send + Sync,
    {
        let (s, r) = async_oneshot::oneshot();
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
    /// Execute the given job and block the current thread until finished.
    /// If you need a non blocking method, see `ThreadPoolAsyncHandler`.
    pub fn execute<F, R>(&self, f: F) -> Result<R, ThreadPoolDisconnected>
    where
        F: 'static + Send + FnOnce(&Shared) -> R,
        R: 'static + Send,
    {
        let (s, r) = flume::bounded(1);
        self.sender
            .send(MsgForWorker::NewJob(Box::new(move |shared| {
                let _ = s.send(f(shared));
            })))
            .map_err(|_| ThreadPoolDisconnected)?;

        r.recv().map_err(|_| ThreadPoolDisconnected)
    }
    /// Launch the given job and return a oneshot receiver that listen job result.
    /// If you need a non blocking method, see `ThreadPoolAsyncHandler`.
    pub fn launch<F, R>(&self, f: F) -> Result<JoinHandle<R>, ThreadPoolDisconnected>
    where
        F: 'static + Send + FnOnce(&Shared) -> R,
        R: 'static + Send,
    {
        let (s, r) = flume::bounded(1);
        self.sender
            .send(MsgForWorker::NewJob(Box::new(move |shared| {
                let _ = s.send(f(shared));
            })))
            .map_err(|_| ThreadPoolDisconnected)?;

        Ok(JoinHandle(r))
    }
}

#[derive(Debug)]
pub struct JoinHandle<R>(FlumeReceiver<R>);

impl<R> JoinHandle<R> {
    pub fn join(self) -> Result<R, ThreadPoolDisconnected> {
        self.0.recv().map_err(|_| ThreadPoolDisconnected)
    }
}
