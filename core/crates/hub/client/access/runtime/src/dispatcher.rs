use crate::ClientAccessRuntime;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{CLIENT_ACCESS_METHODS, Command, Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

impl Dispatcher for ClientAccessRuntime {
    fn hub_id(&self) -> HubId {
        self.state.hub_id()
    }

    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            let command = &request.fields().command;
            let local = request.fields().destination == self.hub_id();
            let mutating = command.mutates();
            if !local
                && matches!(
                    command,
                    Command::GetClientAccess {} | Command::ConfigureClientAccess { .. }
                )
            {
                return Err(PublicError::Unauthorized);
            }
            let unavailable = if mutating {
                PublicError::OutcomeUnknown
            } else {
                PublicError::Unreachable
            };
            // Check durable and live authorization on every call, including Hello
            // and ordinary reads. The monitor also catches changes while idle.
            let current = self.current().map_err(|_| unavailable)?;
            match command {
                Command::GetClientAccess {} => {
                    return Ok(Operation::Reply(Reply::ClientAccess {
                        configuration: current,
                    }));
                }
                Command::ConfigureClientAccess { change } => {
                    let configuration = self.configure(change)?;
                    return Ok(Operation::Reply(Reply::ClientAccess { configuration }));
                }
                _ => {}
            }
            let hello = local && matches!(command, Command::Hello {});
            let outcome = self.inner.dispatch(request).await;
            self.current().map_err(|_| unavailable)?;
            let mut outcome = outcome?;
            if hello {
                if let Operation::Reply(Reply::Hello { hello }) = &mut outcome {
                    for method in CLIENT_ACCESS_METHODS {
                        if !hello.methods.contains(method) {
                            hello.methods.push(*method);
                        }
                    }
                }
            }
            Ok(outcome)
        })
    }
}
