const TASK_DRAIN_CURRENT_REQUIRED: &str = "task drain current handle is unavailable";

struct TaskDrain {
    handles: Vec<tokio::task::JoinHandle<()>>,
}

impl TaskDrain {
    fn new(handles: Vec<tokio::task::JoinHandle<()>>) -> Self {
        Self { handles }
    }

    fn current(&mut self) -> Option<&mut tokio::task::JoinHandle<()>> {
        self.handles.last_mut()
    }

    fn complete_current(&mut self) {
        drop(self.handles.pop());
    }

    fn abort_all(&self) {
        self.handles.iter().for_each(tokio::task::JoinHandle::abort);
    }
}

impl Drop for TaskDrain {
    fn drop(&mut self) {
        self.abort_all();
    }
}

async fn abort_and_settle(tasks: &mut TaskDrain) -> bool {
    tasks.abort_all();
    let deadline = tokio::time::Instant::now() + TASK_ABORT_SETTLE_TIMEOUT;
    while tasks.current().is_some() {
        let settled = {
            let task = tasks.current().expect(TASK_DRAIN_CURRENT_REQUIRED);
            tokio::time::timeout_at(deadline, task).await.is_ok()
        };
        if !settled {
            return false;
        }
        tasks.complete_current();
    }
    true
}
