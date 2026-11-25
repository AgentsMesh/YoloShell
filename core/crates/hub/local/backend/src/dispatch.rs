use crate::{Attachment, LocalBackend};
use yolo_hub_dispatch::{Dispatcher, EventFuture, Operation, OperationFuture, TerminalSource};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Request};
use yolo_hub_terminal_protocol::{AttachResult, PublicError};

impl Dispatcher for LocalBackend {
    fn hub_id(&self) -> HubId {
        self.hub_id()
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            if request.fields().destination != self.hub {
                return Err(PublicError::Unreachable);
            }
            match &request.fields().command {
                Command::Attach { request } => {
                    Ok(Operation::Attached(Box::new(self.attach(*request).await?)))
                }
                command => self.execute(command).await.map(Operation::Reply),
            }
        })
    }
}
impl TerminalSource for Attachment {
    fn initial(&self) -> &AttachResult {
        &self.initial
    }
    fn next_event(&mut self) -> EventFuture<'_> {
        Box::pin(Attachment::next_event(self))
    }
}
