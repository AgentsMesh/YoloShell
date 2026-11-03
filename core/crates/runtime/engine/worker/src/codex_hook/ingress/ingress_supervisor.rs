use super::authority_registry::CodexAuthorityRegistry;
use super::constants::{
    CODEX_INGRESS_ABORT_TIMEOUT, CODEX_INGRESS_DRAIN_TIMEOUT, CODEX_INGRESS_LISTENER_DRAIN_ERROR,
    CODEX_INGRESS_SHUTDOWN_ERROR, CODEX_INGRESS_TASK_NAME,
};
use super::ingress_operation::IngressOperation;
use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

pub(in crate::worker) struct CodexHookIngressSupervisor {
    registry: Arc<CodexAuthorityRegistry>,
    inner: Arc<IngressSupervisorInner>,
    projection_completion: super::projection_completion::ProjectionCompletion,
}

pub(super) struct IngressSupervisorInner {
    state: Mutex<IngressState>,
    listener: Mutex<Option<IngressListener>>,
    publication_gate: Arc<tokio::sync::Mutex<()>>,
    projection_router: Mutex<Option<Arc<super::router::CodexSessionRouter>>>,
    #[cfg(test)]
    start_gate: Mutex<Option<Arc<IngressStartGate>>>,
    #[cfg(test)]
    test_control: Arc<IngressTestControl>,
}

enum IngressState {
    Idle,
    Starting(Arc<IngressOperation>),
    Running(Arc<IngressOperation>),
    Restarting(Arc<IngressOperation>),
    Stopping(Arc<IngressOperation>),
    Finished(Arc<IngressOperation>),
}

struct IngressListener {
    identity: Arc<()>,
    abort: tokio::task::AbortHandle,
    completion: super::ingress_listener_completion::IngressListenerCompletion,
}

#[cfg(test)]
pub(in crate::worker::codex_hook) struct IngressStartGate {
    entered: tokio::sync::Notify,
    released: tokio::sync::Notify,
}

#[cfg(test)]
pub(in crate::worker) struct CodexRegistryLifetimeProbe {
    registry: std::sync::Weak<CodexAuthorityRegistry>,
}

impl CodexHookIngressSupervisor {
    pub(in crate::worker) fn new() -> Self {
        Self {
            registry: Arc::new(CodexAuthorityRegistry::default()),
            projection_completion: Default::default(),
            inner: Arc::new(IngressSupervisorInner {
                state: Mutex::new(IngressState::Idle),
                listener: Mutex::new(None),
                publication_gate: Arc::new(tokio::sync::Mutex::new(())),
                projection_router: Mutex::new(None),
                #[cfg(test)]
                start_gate: Mutex::new(None),
                #[cfg(test)]
                test_control: Arc::new(IngressTestControl::default()),
            }),
        }
    }

    pub(in crate::worker) async fn shutdown(&self) -> Result<()> {
        let publication_guard = Arc::clone(&self.inner.publication_gate).lock_owned().await;
        let Some(operation) = self.inner.begin_shutdown() else {
            return Ok(());
        };
        operation.cancel();
        drop(publication_guard);
        let settlement = tokio::time::timeout(CODEX_INGRESS_DRAIN_TIMEOUT, operation.wait()).await;
        let listener_completion = self.inner.abort_listener(&operation);
        if settlement.is_err() {
            operation.abort();
        }
        self.clear_projection_router();
        if let Some(completion) = listener_completion {
            tokio::time::timeout(CODEX_INGRESS_ABORT_TIMEOUT, completion.wait())
                .await
                .context(CODEX_INGRESS_LISTENER_DRAIN_ERROR)?;
        }
        self.inner.clear(&operation);
        let settlement = settlement.context(CODEX_INGRESS_SHUTDOWN_ERROR)?;
        settlement.into_result()
    }

    #[cfg(test)]
    pub(in crate::worker) fn registry_probe(&self) -> CodexRegistryLifetimeProbe {
        CodexRegistryLifetimeProbe {
            registry: Arc::downgrade(&self.registry),
        }
    }

    #[cfg(test)]
    pub(in crate::worker::codex_hook) fn gate_next_start_for_test(&self) -> Arc<IngressStartGate> {
        let gate = Arc::new(IngressStartGate {
            entered: tokio::sync::Notify::new(),
            released: tokio::sync::Notify::new(),
        });
        let previous = self
            .inner
            .start_gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .replace(Arc::clone(&gate));
        assert!(previous.is_none());
        gate
    }
}

pub(super) fn report_task_failure(detail: &str) {
    let failure = crate::worker::supervision::TaskFailure::message(CODEX_INGRESS_TASK_NAME, detail);
    crate::worker::supervision::report_contained(&failure);
}

#[cfg(test)]
impl CodexRegistryLifetimeProbe {
    pub(in crate::worker) fn shares_allocation(&self, other: &Self) -> bool {
        std::sync::Weak::ptr_eq(&self.registry, &other.registry)
    }

    pub(in crate::worker) fn is_alive(&self) -> bool {
        self.registry.upgrade().is_some()
    }
}

include!("ingress_supervisor_state.rs");
include!("ingress_supervisor_bind.rs");
include!("ingress_supervisor_run.rs");
include!("ingress_supervisor_restart.rs");
include!("ingress_projection.rs");
include!("ingress_control.rs");
#[cfg(all(test, feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-runtime"))]
#[path = "../../../unit_tests/codex_hook/ingress/ingress_shutdown_result_tests.rs"]
mod shutdown_result_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
include!("../../../test_support/unit/codex_hook/ingress/ingress_supervisor_test_support.rs");
