use crate::{ManagedHub, ManagementRequest};
use tokio::sync::{mpsc::error::TrySendError, oneshot};
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{
    BUILD_METHODS, Command, HUB_PROTOCOL_VERSION, Hello, MANAGEMENT_METHODS, Reply, Request,
    SUPPORTED_METHODS,
};
use yolo_hub_terminal_protocol::PublicError;

impl Dispatcher for ManagedHub {
    fn hub_id(&self) -> HubId {
        self.state.hub_id()
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            self.state.verify().map_err(|_| PublicError::Unreachable)?;
            let fields = request.fields();
            if matches!(
                fields.command,
                Command::GetClientAccess {}
                    | Command::ConfigureClientAccess { .. }
                    | Command::GetSshProfiles {}
                    | Command::ConfigureSshProfiles { .. }
                    | Command::ConnectSshProfile { .. }
                    | Command::RemoteSetup { .. }
                    | Command::InspectRemoteSetup { .. }
            ) {
                return Err(PublicError::Unsupported);
            }
            if matches!(fields.command, Command::GetBuild {}) {
                if fields.destination != self.hub_id() {
                    return Err(PublicError::Unauthorized);
                }
                let build = self
                    .build
                    .as_ref()
                    .ok_or(PublicError::Unsupported)?
                    .build()
                    .await?;
                self.state.verify().map_err(|_| PublicError::Unreachable)?;
                return Ok(Operation::Reply(Reply::Build { build }));
            }
            if fields.command.local_only() {
                if fields.destination != self.hub_id() {
                    return Err(PublicError::Unauthorized);
                }
                if matches!(fields.command, Command::ListConsumers {}) {
                    let generation = self.generation.read().await;
                    let federation = generation.as_ref().ok_or(PublicError::Unsupported)?;
                    return Ok(Operation::Reply(Reply::Consumers {
                        consumers: federation.consumers(),
                    }));
                }
                let change = match &fields.command {
                    Command::Configure { change } => Some(change.clone()),
                    _ => None,
                };
                let (reply, result) = oneshot::channel();
                self.commands
                    .try_send(ManagementRequest { change, reply })
                    .map_err(|error| match error {
                        TrySendError::Full(_) => PublicError::Conflict,
                        TrySendError::Closed(_) => PublicError::Unreachable,
                    })?;
                let snapshot = result.await.map_err(|_| {
                    if fields.command.mutates() {
                        PublicError::OutcomeUnknown
                    } else {
                        PublicError::Unreachable
                    }
                })??;
                return Ok(Operation::Reply(Reply::Configuration { snapshot }));
            }
            if fields.destination == self.hub_id() && matches!(fields.command, Command::Hello {}) {
                let mut methods = SUPPORTED_METHODS.to_vec();
                methods.extend_from_slice(MANAGEMENT_METHODS);
                if self.build.is_some() {
                    methods.extend_from_slice(BUILD_METHODS);
                }
                return Ok(Operation::Reply(Reply::Hello {
                    hello: Hello {
                        hub_id: self.hub_id(),
                        version: HUB_PROTOCOL_VERSION,
                        methods,
                    },
                }));
            }
            // Keep the generation guard for the whole operation; returned sources own no node.
            let generation = self.generation.read().await;
            generation
                .as_ref()
                .ok_or(PublicError::Unreachable)?
                .dispatch(request)
                .await
        })
    }
}
