use super::cancellation::IngressCancellation;
use super::constants::{CODEX_INGRESS_TASK_ERROR, CODEX_INGRESS_UNSETTLED_PERMITS};
use super::ingress_supervisor::IngressSupervisorInner;
use anyhow::{Result, bail};
use std::future::Future;
use std::sync::{Arc, Mutex, Weak};

#[derive(Clone)]
pub(super) enum IngressSettlement {
    Succeeded,
    Cancelled,
    Failed(Arc<str>),
}

pub(super) struct IngressOperation {
    identity: Arc<()>,
    cancellation: IngressCancellation,
    abort: tokio::task::AbortHandle,
    completion: Arc<IngressCompletion>,
    monitor: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

struct IngressCompletion {
    settlement: Mutex<Option<IngressSettlement>>,
    signal: tokio::sync::Semaphore,
}

impl IngressOperation {
    pub(super) fn spawn<Factory, Operation>(
        inner: Weak<IngressSupervisorInner>,
        factory: Factory,
    ) -> (Arc<Self>, tokio::sync::oneshot::Sender<()>)
    where
        Factory: FnOnce(IngressCancellation, Arc<()>) -> Operation + Send + 'static,
        Operation: Future<Output = Result<()>> + Send + 'static,
    {
        let identity = Arc::new(());
        let cancellation = IngressCancellation::default();
        let (launch, launched) = tokio::sync::oneshot::channel();
        let operation = factory(cancellation.clone(), Arc::clone(&identity));
        let task = tokio::spawn(async move {
            if launched.await.is_err() {
                return Ok(());
            }
            operation.await
        });
        let abort = task.abort_handle();
        let completion = Arc::new(IngressCompletion::new());
        let monitor_completion = Arc::clone(&completion);
        let monitor_identity = Arc::clone(&identity);
        let monitor = tokio::spawn(async move {
            let settlement = IngressSettlement::from_join(task.await);
            if let IngressSettlement::Failed(detail) = &settlement {
                super::ingress_supervisor::report_task_failure(detail);
            }
            if let Some(inner) = inner.upgrade() {
                inner.mark_finished(&monitor_identity);
            }
            monitor_completion.publish(settlement);
        });
        let operation = Arc::new(Self {
            identity,
            cancellation,
            abort,
            completion,
            monitor: Mutex::new(Some(monitor)),
        });
        (operation, launch)
    }

    pub(super) fn matches(&self, identity: &Arc<()>) -> bool {
        Arc::ptr_eq(&self.identity, identity)
    }

    pub(super) fn cancel(&self) {
        self.cancellation.cancel();
    }

    pub(super) fn control_fence(&self) -> Option<super::cancellation::IngressGenerationFence> {
        self.cancellation.try_fence()
    }

    pub(super) fn abort(&self) {
        self.abort.abort();
    }

    pub(super) fn is_complete(&self) -> bool {
        self.completion.is_complete()
    }

    pub(super) async fn wait(&self) -> IngressSettlement {
        self.completion.wait().await
    }
}

impl IngressCompletion {
    fn new() -> Self {
        Self {
            settlement: Mutex::new(None),
            signal: tokio::sync::Semaphore::new(CODEX_INGRESS_UNSETTLED_PERMITS),
        }
    }

    fn publish(&self, settlement: IngressSettlement) {
        *self
            .settlement
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(settlement);
        self.signal.close();
    }

    fn is_complete(&self) -> bool {
        self.signal.is_closed()
    }

    async fn wait(&self) -> IngressSettlement {
        let _ = self.signal.acquire().await;
        self.settlement
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .unwrap_or_else(|| IngressSettlement::Failed(Arc::from(CODEX_INGRESS_TASK_ERROR)))
    }
}

impl IngressSettlement {
    fn from_join(joined: std::result::Result<Result<()>, tokio::task::JoinError>) -> Self {
        match joined {
            Ok(Ok(())) => Self::Succeeded,
            Ok(Err(error)) => Self::Failed(Arc::from(format!("{error:#}"))),
            Err(error) if error.is_cancelled() => Self::Cancelled,
            Err(error) => Self::Failed(Arc::from(format!("{CODEX_INGRESS_TASK_ERROR}: {error}"))),
        }
    }

    pub(super) fn into_result(self) -> Result<()> {
        match self {
            Self::Succeeded | Self::Cancelled => Ok(()),
            Self::Failed(detail) => bail!(detail.to_string()),
        }
    }
}

impl Drop for IngressOperation {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.abort.abort();
        if let Some(monitor) = self
            .monitor
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take()
        {
            monitor.abort();
        }
    }
}
