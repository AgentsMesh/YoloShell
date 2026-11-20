use std::{
    collections::BTreeMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
};
use tokio::task::{AbortHandle, JoinHandle};

/// The connection owns cancellation even if Cap'n Proto retains an outstanding method promise.
#[derive(Default)]
pub(crate) struct Scope {
    tasks: Mutex<BTreeMap<u64, AbortHandle>>,
    next: AtomicU64,
    closed: AtomicBool,
}
impl Scope {
    pub fn spawn<T: Send + 'static>(
        self: &Arc<Self>,
        runtime: &tokio::runtime::Handle,
        future: impl std::future::Future<Output = T> + Send + 'static,
    ) -> Result<Task<T>, crate::Error> {
        let mut tasks = self.tasks.lock().unwrap();
        if self.closed.load(Ordering::Acquire) {
            return Err(crate::Error::Closed);
        }
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let handle = runtime.spawn(future);
        tasks.insert(id, handle.abort_handle());
        Ok(Task {
            handle,
            id,
            scope: self.clone(),
        })
    }
    pub fn cancel(&self) {
        let mut tasks = self.tasks.lock().unwrap();
        self.closed.store(true, Ordering::Release);
        for (_, task) in std::mem::take(&mut *tasks) {
            task.abort();
        }
    }
}
pub(crate) struct Task<T> {
    pub handle: JoinHandle<T>,
    id: u64,
    scope: Arc<Scope>,
}
impl<T> Drop for Task<T> {
    fn drop(&mut self) {
        self.handle.abort();
        self.scope.tasks.lock().unwrap().remove(&self.id);
    }
}
pub(crate) struct Guard(pub Arc<Scope>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.cancel();
    }
}
