use std::{
    future::Future,
    sync::{Arc, Condvar, Mutex},
};
use tokio::{
    sync::oneshot,
    task::AbortHandle,
    time::{Duration, timeout},
};

pub const TEST_TIMEOUT: Duration = Duration::from_secs(5);
pub const PIPE_BYTES: usize = 65_536;
const THREAD_NAME: &str = "hub-generation-drain-test";
const TEARDOWN_NOT_RELEASED: &str = "test did not release generation teardown";

pub struct DropGate {
    entered: Option<oneshot::Sender<()>>,
    released: Arc<(Mutex<bool>, Condvar)>,
}
pub struct Gate {
    pub entered: oneshot::Receiver<()>,
    released: Arc<(Mutex<bool>, Condvar)>,
}
impl Gate {
    pub fn new() -> (Self, DropGate) {
        let (entered, receiver) = oneshot::channel();
        let released = Arc::new((Mutex::new(false), Condvar::new()));
        (
            Self {
                entered: receiver,
                released: released.clone(),
            },
            DropGate {
                entered: Some(entered),
                released,
            },
        )
    }
    pub fn release(&self) {
        *self.released.0.lock().unwrap() = true;
        self.released.1.notify_all();
    }
}
impl Drop for Gate {
    fn drop(&mut self) {
        self.release();
    }
}
impl Drop for DropGate {
    fn drop(&mut self) {
        let _ = self.entered.take().unwrap().send(());
        let released = self.released.0.lock().unwrap();
        let (released, _) = self
            .released
            .1
            .wait_timeout_while(released, TEST_TIMEOUT, |released| !*released)
            .unwrap();
        assert!(*released, "{TEARDOWN_NOT_RELEASED}");
    }
}

pub struct DomainTask {
    pub abort: AbortHandle,
    finished: oneshot::Receiver<()>,
    thread: std::thread::JoinHandle<()>,
}
impl DomainTask {
    pub async fn spawn(task: impl Future<Output = ()> + Send + 'static) -> Self {
        let (ready, started) = oneshot::channel();
        let (done, finished) = oneshot::channel();
        let thread = std::thread::Builder::new()
            .name(THREAD_NAME.into())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async move {
                    let task = tokio::spawn(task);
                    ready.send(task.abort_handle()).unwrap();
                    let _ = task.await;
                });
                drop(runtime);
                let _ = done.send(());
            })
            .unwrap();
        Self {
            abort: timeout(TEST_TIMEOUT, started).await.unwrap().unwrap(),
            finished,
            thread,
        }
    }
    pub async fn finish(self) {
        timeout(TEST_TIMEOUT, self.finished).await.unwrap().unwrap();
        self.thread.join().unwrap();
    }
}
