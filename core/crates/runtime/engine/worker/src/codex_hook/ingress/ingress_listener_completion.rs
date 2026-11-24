use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone)]
pub(super) struct IngressListenerCompletion {
    state: Arc<CompletionState>,
}

pub(super) struct IngressListenerCompletionGuard {
    completion: IngressListenerCompletion,
}

struct CompletionState {
    completed: AtomicBool,
    changed: tokio::sync::Notify,
}

impl IngressListenerCompletion {
    pub(super) fn pending() -> (Self, IngressListenerCompletionGuard) {
        let completion = Self {
            state: Arc::new(CompletionState {
                completed: AtomicBool::new(false),
                changed: tokio::sync::Notify::new(),
            }),
        };
        let guard = IngressListenerCompletionGuard {
            completion: completion.clone(),
        };
        (completion, guard)
    }

    pub(super) async fn wait(&self) {
        loop {
            let changed = self.state.changed.notified();
            tokio::pin!(changed);
            changed.as_mut().enable();
            if self.state.completed.load(Ordering::Acquire) {
                return;
            }
            changed.await;
        }
    }

    fn complete(&self) {
        if !self.state.completed.swap(true, Ordering::AcqRel) {
            self.state.changed.notify_waiters();
        }
    }
}

impl Drop for IngressListenerCompletionGuard {
    fn drop(&mut self) {
        self.completion.complete();
    }
}
