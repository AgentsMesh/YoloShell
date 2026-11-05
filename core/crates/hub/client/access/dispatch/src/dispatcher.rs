use crate::{admission, source::ScopedSource, while_authorized};
use std::sync::Arc;
use yolo_hub_client_access_registry::ClientAdmission;
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply, Request, Response};
use yolo_hub_terminal_protocol::PublicError;

/// A public client principal can only reduce the entry Hub's delegated rights.
/// It does not gain local management authority from the forwarding socket's UID.
pub struct ScopedDispatcher {
    pub(crate) inner: Arc<dyn Dispatcher>,
    pub(crate) admission: ClientAdmission,
}

impl ScopedDispatcher {
    pub fn new(
        inner: Arc<dyn Dispatcher>,
        admission: ClientAdmission,
    ) -> Result<Self, PublicError> {
        if inner.hub_id() != admission.entry_hub() {
            return Err(PublicError::OriginMismatch);
        }
        crate::guard::verify(&admission)?;
        Ok(Self { inner, admission })
    }

    async fn execute(&self, request: Request) -> Result<Operation, PublicError> {
        admission::authorize(&self.admission, &request)?;
        if let Some(reply) = self.catalogue(&request).await? {
            return Ok(Operation::Reply(reply));
        }
        match self.inner.dispatch(request.clone()).await? {
            Operation::Reply(mut reply) => {
                Response::new(&request, request.fields().destination, reply.clone())
                    .validate_for(&request)?;
                if let (Command::Hello {}, Reply::Hello { hello }) =
                    (&request.fields().command, &mut reply)
                {
                    hello
                        .methods
                        .retain(|method| admission::advertised(&self.admission, *method));
                }
                Ok(Operation::Reply(reply))
            }
            Operation::Attached(source) => {
                Response::new(
                    &request,
                    request.fields().destination,
                    Reply::Attached {
                        attachment: source.initial().clone(),
                    },
                )
                .validate_for(&request)?;
                Ok(Operation::Attached(Box::new(ScopedSource {
                    inner: source,
                    admission: self.admission.clone(),
                })))
            }
        }
    }
}

impl Dispatcher for ScopedDispatcher {
    fn hub_id(&self) -> HubId {
        self.inner.hub_id()
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(while_authorized(&self.admission, self.execute(request)))
    }
}
