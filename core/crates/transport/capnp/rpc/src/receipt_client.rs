use crate::{Confirmation, Connection, Error, Future};
use tokio::{
    sync::{oneshot, watch},
    time::{Instant, timeout_at},
};
use yolo_capnp_schema::rpc_capnp::receipt;

pub struct DeferredReceipt {
    pub(crate) sender: oneshot::Sender<ConfirmRequest>,
    pub(crate) connection: Connection,
}
pub(crate) struct ConfirmRequest {
    connection: Connection,
    deadline: Instant,
    response: oneshot::Sender<Result<(), Error>>,
}
impl DeferredReceipt {
    /// Loss or expiry is uncertain; this capability is never replayed automatically.
    pub async fn confirm(self, deadline: Instant) -> Result<(), Error> {
        let (response, completion) = oneshot::channel();
        self.sender
            .send(ConfirmRequest {
                connection: self.connection,
                deadline,
                response,
            })
            .map_err(|_| Error::OutcomeUnknown)?;
        timeout_at(deadline, completion)
            .await
            .map_err(|_| Error::OutcomeUnknown)?
            .map_err(|_| Error::OutcomeUnknown)?
    }
}
impl Confirmation for DeferredReceipt {
    fn confirm(self: Box<Self>) -> Future<'static, ()> {
        Box::pin(async move {
            (*self)
                .confirm(Instant::now() + crate::MAX_RECEIPT_WAIT)
                .await
        })
    }
}

pub(crate) fn defer(
    client: receipt::Client,
    mut stop: watch::Receiver<bool>,
) -> oneshot::Sender<ConfirmRequest> {
    let (sender, confirmation) = oneshot::channel::<ConfirmRequest>();
    tokio::task::spawn_local(async move {
        let request = tokio::select! {
            _ = stop.changed() => return,
            result = confirmation => match result { Ok(request) => request, Err(_) => return },
        };
        let _retained = request.connection;
        let result = timeout_at(request.deadline, client.confirm_request().send().promise)
            .await
            .map_err(|_| Error::OutcomeUnknown)
            .and_then(|reply| reply.map(|_| ()).map_err(|_| Error::OutcomeUnknown));
        let _ = request.response.send(result);
    });
    sender
}
