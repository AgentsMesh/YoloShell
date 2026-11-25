use crate::{
    RelayError, Status,
    authority::retire_expired,
    control::{Control, request_deadline},
};
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};
use yolo_hub_agent_protocol::{CommandReceipt, FailureCode};
use yolo_hub_client::ClientError;
use yolo_hub_relay_control_protocol::{NotSentReason, Outcome, Request, Response};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

pub struct ControlCommand {
    pub request: Request,
    pub response: oneshot::Sender<Response>,
}

pub struct CommandChannel {
    pub launch_id: uuid::Uuid,
    pub incoming: mpsc::Receiver<ControlCommand>,
}

pub(crate) struct SemanticOperation {
    pub command: ControlCommand,
    pub public: yolo_hub_agent_protocol::AgentControlRequest,
    pub grant: yolo_hub_terminal_protocol::LeaseGrant,
    pub deadline: Instant,
}

pub(crate) async fn next(commands: &mut Option<CommandChannel>) -> Option<ControlCommand> {
    match commands {
        Some(channel) => channel.incoming.recv().await,
        None => std::future::pending().await,
    }
}

impl Control {
    /// Compatibility entry point for direct unit-level dispatch.  The relay
    /// loop uses `begin_semantic` and `finish_semantic` so this RPC cannot
    /// block renewal or the PTY writer lane.
    pub(crate) async fn semantic_interrupt(
        &mut self,
        command: ControlCommand,
        launch: uuid::Uuid,
        status: &mpsc::Sender<Status>,
    ) -> Result<(), RelayError> {
        let Some(operation) = self.begin_semantic(command, launch, status).await? else {
            return Ok(());
        };
        let mut client = self.take_call_client().await?;
        let result = client
            .call(
                self.origin(),
                Command::AgentControl {
                    request: operation.public.clone(),
                    grant: operation.grant,
                },
                operation.deadline,
            )
            .await;
        let client_usable = client.is_usable();
        self.return_call_client(client);
        self.finish_semantic(operation, client_usable, result, status)
            .await
    }

    pub(crate) async fn begin_semantic(
        &mut self,
        command: ControlCommand,
        launch: uuid::Uuid,
        status: &mpsc::Sender<Status>,
    ) -> Result<Option<SemanticOperation>, RelayError> {
        if command.response.is_closed() {
            return Ok(None);
        }
        let outcome = if command.request.fields().launch_id != launch
            || command.request.fields().terminal != self.options.target
        {
            Some(Outcome::NotSent {
                reason: NotSentReason::TargetChanged,
            })
        } else if retire_expired(&mut self.authority, Instant::now()) {
            self.lost(status).await?;
            Some(Outcome::NotSent {
                reason: NotSentReason::Expired,
            })
        } else if let Some(authority) = &self.authority {
            if authority.grant.fields().target != self.options.target
                || authority.grant.fields().principal != self.options.principal
            {
                Some(Outcome::NotSent {
                    reason: NotSentReason::TargetChanged,
                })
            } else {
                let operation = SemanticOperation {
                    public: command.request.fields().request.clone(),
                    grant: authority.grant,
                    deadline: request_deadline(authority),
                    command,
                };
                return Ok(Some(operation));
            }
        } else {
            Some(Outcome::NotSent {
                reason: NotSentReason::NoWriter,
            })
        };
        let outcome = outcome.expect("immediate semantic outcome");
        let response =
            Response::new(&command.request, outcome).map_err(|_| RelayError::InvalidReply)?;
        let _ = command.response.send(response);
        Ok(None)
    }

    pub(crate) async fn finish_semantic(
        &mut self,
        operation: SemanticOperation,
        client_usable: bool,
        result: Result<Reply, ClientError>,
        status: &mpsc::Sender<Status>,
    ) -> Result<(), RelayError> {
        let outcome = match result {
            Ok(Reply::AgentControl { receipt }) => Outcome::Receipt { receipt },
            Ok(_) => {
                self.authority = None;
                self.lost(status).await?;
                Outcome::Unknown {}
            }
            Err(error) => {
                let outcome = failure(&operation.public, &error);
                if !client_usable
                    || matches!(
                        error,
                        ClientError::Remote(
                            PublicError::Conflict
                                | PublicError::Unauthorized
                                | PublicError::TargetMismatch
                        )
                    )
                {
                    self.authority = None;
                    self.lost(status).await?;
                }
                outcome
            }
        };
        let response = Response::new(&operation.command.request, outcome)
            .map_err(|_| RelayError::InvalidReply)?;
        let _ = operation.command.response.send(response);
        Ok(())
    }
}

fn failure(request: &yolo_hub_agent_protocol::AgentControlRequest, error: &ClientError) -> Outcome {
    if error.outcome_is_unknown() {
        return Outcome::Unknown {};
    }
    let code = match error {
        ClientError::Remote(PublicError::Unauthorized) => FailureCode::PermissionDenied,
        ClientError::Remote(PublicError::Unsupported) => FailureCode::Unsupported,
        ClientError::Remote(PublicError::Conflict | PublicError::TargetMismatch) => {
            FailureCode::PreconditionsChanged
        }
        ClientError::Remote(_) => FailureCode::AdapterRejected,
        ClientError::Unsupported
        | ClientError::Validation(_)
        | ClientError::Poisoned
        | ClientError::TerminalStream
        | ClientError::WrongLocalUser => {
            return Outcome::NotSent {
                reason: NotSentReason::Unavailable,
            };
        }
        _ => return Outcome::Unknown {},
    };
    Outcome::Receipt {
        receipt: CommandReceipt::rejected(request, code),
    }
}
