use crate::{AuditLog, AuditOutcome, should_audit};
use std::sync::Arc;
use yolo_hub_dispatch::{Dispatcher, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Request;
use yolo_hub_terminal_protocol::PublicError;

/// Records audited local mutations; a logging failure never changes the
/// operation outcome.
pub struct AuditedDispatcher {
    inner: Arc<dyn Dispatcher>,
    log: Arc<AuditLog>,
    principal: String,
}

impl AuditedDispatcher {
    pub fn new(inner: Arc<dyn Dispatcher>, log: Arc<AuditLog>, principal: &str) -> Self {
        Self {
            inner,
            log,
            principal: principal.into(),
        }
    }
}

impl Dispatcher for AuditedDispatcher {
    fn hub_id(&self) -> HubId {
        self.inner.hub_id()
    }

    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            let command = request.fields().command.clone();
            let destination = request.fields().destination;
            let result = self.inner.dispatch(request).await;
            if should_audit(&command) {
                let outcome = match &result {
                    Ok(_) => AuditOutcome::Accepted,
                    Err(PublicError::Unauthorized) => AuditOutcome::Denied,
                    Err(_) => AuditOutcome::Failed,
                };
                let _ = self
                    .log
                    .record(&self.principal, destination, &command, outcome);
            }
            result
        })
    }
}
