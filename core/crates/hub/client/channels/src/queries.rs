use crate::{ControlError, ControlRequest, Endpoint, constants::*, operations::Completed};
use std::{collections::HashMap, future::Future};
use tokio::{
    sync::watch,
    task::{Id, JoinSet},
    time::Instant,
};
use uuid::Uuid;
use yolo_hub_rpc_protocol::Reply;

/// Queries own sockets and task slots independently of writers and lifecycle work.
/// Keeping requests outside their tasks preserves correlation through a panic.
/// Dropping the JoinSet aborts every owned query and drops its private socket.
#[derive(Default)]
pub(crate) struct Queries {
    tasks: JoinSet<Result<Reply, ControlError>>,
    requests: HashMap<Id, ControlRequest>,
}

impl Queries {
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn start(
        &mut self,
        endpoint: &Endpoint,
        request: &ControlRequest,
        generation: watch::Receiver<Option<Uuid>>,
    ) -> Result<(), ControlError> {
        if self.tasks.len() >= MAX_QUERY_CHANNELS {
            return Err(ControlError::QueryLimit);
        }
        let endpoint = endpoint.clone();
        let task_request = request.clone();
        let deadline = Instant::now() + request.command.operation_timeout(RPC_TIMEOUT);
        let deadline = request.validity.as_ref().map_or(deadline, |validity| {
            deadline.min(Instant::from_std(validity.deadline()))
        });
        self.spawn(request, async move {
            crate::operations::execute(endpoint, &task_request, generation, deadline).await
        });
        Ok(())
    }

    fn spawn(
        &mut self,
        request: &ControlRequest,
        task: impl Future<Output = Result<Reply, ControlError>> + Send + 'static,
    ) {
        let id = self.tasks.spawn(task).id();
        self.requests.insert(id, request.clone());
    }

    pub async fn next(&mut self) -> Option<Completed> {
        let (id, result) = match self.tasks.join_next_with_id().await? {
            Ok((id, result)) => (id, result),
            Err(error) => (error.id(), Err(ControlError::QueryTaskFailed)),
        };
        Some(Completed {
            request: self
                .requests
                .remove(&id)
                .expect(QUERY_TASK_IDENTITY_MISSING),
            result,
        })
    }
}

#[cfg(test)]
#[path = "../unit_tests/query_tasks.rs"]
mod tests;
