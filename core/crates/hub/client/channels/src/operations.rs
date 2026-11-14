use crate::{ControlError, ControlRequest, Endpoint, admission::admit, constants::*};
use tokio::{
    sync::watch,
    task::{JoinError, JoinSet},
    time::Instant,
};
use uuid::Uuid;
use yolo_hub_rpc_protocol::Reply;

pub(crate) struct Completed {
    pub request: ControlRequest,
    pub result: Result<Reply, ControlError>,
}

/// Slow commands own separate channels, so they cannot occupy the writer lane.
/// Dropping this owner cancels every task and drops its independently owned I/O.
#[derive(Default)]
pub(crate) struct Operations {
    tasks: JoinSet<Completed>,
}

impl Operations {
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn start(
        &mut self,
        endpoint: &Endpoint,
        request: &ControlRequest,
        generation: watch::Receiver<Option<Uuid>>,
    ) -> Result<(), ControlError> {
        if self.tasks.len() >= MAX_LIFECYCLE_CHANNELS {
            return Err(ControlError::OperationLimit);
        }
        let endpoint = endpoint.clone();
        let request = request.clone();
        let budget = request.command.operation_timeout(RPC_TIMEOUT);
        let deadline = request
            .validity
            .as_ref()
            .map_or(Instant::now() + budget, |validity| {
                (Instant::now() + budget).min(Instant::from_std(validity.deadline()))
            });
        self.tasks.spawn(async move {
            let result = execute(endpoint, &request, generation, deadline).await;
            Completed { request, result }
        });
        Ok(())
    }

    pub async fn next(&mut self) -> Option<Result<Completed, JoinError>> {
        self.tasks.join_next().await
    }
}

pub(crate) async fn execute(
    endpoint: Endpoint,
    request: &ControlRequest,
    mut generation: watch::Receiver<Option<Uuid>>,
    deadline: Instant,
) -> Result<Reply, ControlError> {
    validate(request, &generation)?;
    let opening = endpoint
        .entry
        .connect(endpoint.hub, deadline.min(Instant::now() + RPC_TIMEOUT));
    let mut client = tokio::select! {
        biased;
        _ = generation.changed() => return Err(ControlError::StaleGeneration),
        result = opening => result?,
    };
    validate(request, &generation)?;
    // Once dispatch begins, preserve its exact result, including uncertainty.
    // A later control generation cannot authorize replay or reclaim this result.
    client
        .call(request.destination, request.command.clone(), deadline)
        .await
        .map_err(Into::into)
}

fn validate(
    request: &ControlRequest,
    generation: &watch::Receiver<Option<Uuid>>,
) -> Result<(), ControlError> {
    admit(*generation.borrow(), request.generation)?;
    if request
        .validity
        .as_ref()
        .is_some_and(|validity| !validity.is_current())
    {
        return Err(ControlError::Invalidated);
    }
    Ok(())
}
