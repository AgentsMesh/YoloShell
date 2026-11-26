use crate::{Job, RemoteSetupRuntime, constants::*};
use std::future::Future;
use std::sync::Arc;
use tokio::{sync::watch, time::Instant};
use yolo_hub_dispatch::Operation;
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, RemoteSetupRequest, RemoteSetupStatus};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;

impl RemoteSetupRuntime {
    pub(crate) fn enqueue<F, Fut>(
        &self,
        request: RemoteSetupRequest,
        operation: F,
    ) -> Result<Operation, PublicError>
    where
        F: FnOnce(Instant) -> Fut,
        Fut: Future<Output = anyhow::Result<RemoteSetupOutcome>> + Send + 'static,
    {
        let status = RemoteSetupStatus {
            request: request.clone(),
            outcome: RemoteSetupOutcome::Running {},
        };
        let id = request.request.fields().request_id;
        let (progress, receiver) = watch::channel(status.clone());
        let mut jobs = self.jobs.lock().map_err(|_| PublicError::Unreachable)?;
        crate::task_retention::prune(&mut jobs);
        if jobs.values().filter(|job| !job.completed()).count() >= MAX_ACTIVE_SETUPS
            || jobs.get(&id).is_some_and(|job| !job.completed())
        {
            return Err(PublicError::Conflict);
        }
        jobs.remove(&id); // Reinspection may replace only this request's completed observation.
        let deadline =
            Instant::now() + yolo_remote_provisioning_protocol::PROVISION_OPERATION_TIMEOUT;
        let future = operation(deadline);
        let retention = Arc::downgrade(&self.jobs);
        let admitted_at = Instant::now();
        let handle = tokio::spawn(async move {
            let outcome = match tokio::time::timeout_at(deadline, future).await {
                Ok(Ok(outcome)) => outcome,
                Ok(Err(error)) => {
                    tracing::warn!(operation_id = %id, failure = ?crate::diagnostics::classify(&error), "{OPERATION_FAILED}");
                    RemoteSetupOutcome::Unknown {}
                }
                Err(_) => {
                    tracing::warn!(operation_id = %id, "{OPERATION_TIMED_OUT}");
                    RemoteSetupOutcome::Unknown {}
                }
            };
            progress.send_replace(RemoteSetupStatus { request, outcome });
            crate::task_retention::finished(retention, id);
        });
        jobs.insert(
            id,
            Job {
                handle,
                status: receiver,
                admitted_at,
                completed_at: None,
            },
        );
        Ok(Operation::Reply(Reply::RemoteSetup { status }))
    }
}
