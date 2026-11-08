use crate::RemoteSetupRuntime;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, RemoteSetupStatus, SetupAction};
use yolo_hub_remote_setup_store::{OpenedSetup, SetupObservation, SetupOperation, SetupRecord};
use yolo_hub_rpc_protocol::{Command, REMOTE_SETUP_METHODS, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

impl Dispatcher for RemoteSetupRuntime {
    fn hub_id(&self) -> yolo_hub_identity::HubId {
        self.engine.state.hub_id()
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            let local = request.fields().destination == self.hub_id();
            if let Command::InspectRemoteSetup { operation_id } = &request.fields().command {
                if !local {
                    return Err(PublicError::Unauthorized);
                }
                return self.inspect_history(*operation_id);
            }
            let Command::RemoteSetup { call } = &request.fields().command else {
                let hello = local && matches!(request.fields().command, Command::Hello {});
                let mut result = self.engine.inner.dispatch(request).await?;
                if hello {
                    if let Operation::Reply(Reply::Hello { hello }) = &mut result {
                        for method in REMOTE_SETUP_METHODS {
                            if !hello.methods.contains(method) {
                                hello.methods.push(*method);
                            }
                        }
                    }
                }
                return Ok(result);
            };
            if !local
                || call.request.request.fields().bootstrap.fields().consumer
                    != self.engine.state.peer_identity()
            {
                return Err(PublicError::Unauthorized);
            }
            call.validate().map_err(|_| PublicError::InvalidRequest)?;
            self.engine
                .state
                .verify()
                .map_err(|_| PublicError::Unreachable)?;
            let id = call.request.request.fields().request_id;
            if call.action == SetupAction::Poll {
                let status = self
                    .jobs
                    .lock()
                    .map_err(|_| PublicError::Unreachable)?
                    .get(&id)
                    .map(|job| job.snapshot());
                let status = match status {
                    Some(status) => self
                        .engine
                        .validate_polled(status, &call.request)
                        .await
                        .map_err(|_| PublicError::Unreachable)?,
                    None => RemoteSetupStatus {
                        request: call.request.clone(),
                        outcome: RemoteSetupOutcome::Unknown {},
                    },
                };
                return Ok(Operation::Reply(Reply::RemoteSetup { status }));
            }
            {
                let jobs = self.jobs.lock().map_err(|_| PublicError::Unreachable)?;
                if let Some(job) = jobs.get(&id) {
                    let status = job.snapshot();
                    if status.request != call.request {
                        return Err(PublicError::Conflict);
                    }
                    if !job.completed() {
                        return Ok(Operation::Reply(Reply::RemoteSetup { status }));
                    }
                }
            }
            let (registration, reservation) = self
                .engine
                .profiles
                .reserve_profile(
                    &call
                        .request
                        .profile()
                        .map_err(|_| PublicError::InvalidRequest)?,
                )
                .map_err(|_| PublicError::Conflict)?;
            if crate::job::profile_matches(&registration, &call.request).is_err() {
                return Err(PublicError::Unauthorized);
            }
            let previous = SetupObservation::observe(&self.engine.root, id)
                .map_err(|_| PublicError::Unreachable)?;
            if previous.is_none() && call.action == SetupAction::Continue {
                return Err(PublicError::InvalidRequest);
            }
            let inspect = call.action == SetupAction::Inspect
                || (call.action == SetupAction::Start && previous.is_some());
            if inspect {
                let Some(observation) = previous else {
                    let status = RemoteSetupStatus {
                        request: call.request.clone(),
                        outcome: RemoteSetupOutcome::Unknown {},
                    };
                    return Ok(Operation::Reply(Reply::RemoteSetup { status }));
                };
                let record = observation.record();
                if record.owner != self.engine.state.peer_identity()
                    || record.request != call.request
                    || record.registration != registration
                {
                    return Err(PublicError::Conflict);
                }
                let engine = self.engine.clone();
                return self.enqueue(call.request.clone(), move |deadline| async move {
                    engine.inspect(observation, reservation, deadline).await
                });
            }
            let proposed = match previous {
                Some(observation) => {
                    let record = observation.record().clone();
                    if record.owner != self.engine.state.peer_identity()
                        || record.request != call.request
                        || record.registration != registration
                    {
                        return Err(PublicError::Conflict);
                    }
                    record
                }
                None => SetupRecord::new(
                    self.engine.state.peer_identity(),
                    call.request.clone(),
                    registration,
                    self.engine
                        .configuration()
                        .await
                        .map_err(|_| PublicError::Unreachable)?,
                )
                .map_err(|_| PublicError::Conflict)?,
            };
            let opened = SetupOperation::open(
                self.engine
                    .root
                    .try_clone()
                    .map_err(|_| PublicError::Unreachable)?,
                proposed,
            )
            .map_err(|_| PublicError::Conflict)?;
            let (operation, fresh) = match opened {
                OpenedSetup::Fresh(operation) => (operation, true),
                OpenedSetup::Existing(operation) => (operation, false),
            };
            let mut action = call.action;
            if !fresh && action == SetupAction::Start {
                action = SetupAction::Inspect;
            }
            let engine = self.engine.clone();
            let credentials = call.credentials.clone().map(|value| value.into_bytes());
            self.enqueue(call.request.clone(), move |deadline| async move {
                engine
                    .run(operation, reservation, action, credentials, deadline)
                    .await
            })
        })
    }
}
