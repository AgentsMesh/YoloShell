use crate::{Connection, Error, Value, constants::*};
use tokio::{
    sync::{OwnedSemaphorePermit, mpsc, oneshot, watch},
    time::{Instant, timeout_at},
};
use yolo_capnp_schema::rpc_capnp::events;

pub struct RemoteEvents {
    pub(crate) sender: mpsc::Sender<Next>,
    pub(crate) _connection: Connection,
}
pub(crate) struct Next {
    deadline: Option<Instant>,
    response: oneshot::Sender<Result<Value, Error>>,
}
impl RemoteEvents {
    pub async fn next(&mut self, deadline: Instant) -> Result<Value, Error> {
        timeout_at(deadline, async {
            let (response, result) = oneshot::channel();
            self.sender
                .send(Next {
                    deadline: Some(deadline),
                    response,
                })
                .await
                .map_err(|_| Error::Closed)?;
            result.await.map_err(|_| Error::Closed)?
        })
        .await
        .map_err(|_| Error::Deadline)?
    }
    /// Idle PTYs have no response timer. Dropping this future still cancels their capability read.
    pub async fn next_unbounded(&mut self) -> Result<Value, Error> {
        let (response, result) = oneshot::channel();
        self.sender
            .send(Next {
                deadline: None,
                response,
            })
            .await
            .map_err(|_| Error::Closed)?;
        result.await.map_err(|_| Error::Closed)?
    }
}

pub(crate) fn spawn(
    client: events::Client,
    mut stopped: watch::Receiver<bool>,
    permit: OwnedSemaphorePermit,
) -> mpsc::Sender<Next> {
    let (sender, mut incoming) = mpsc::channel::<Next>(STREAM_QUEUE);
    tokio::task::spawn_local(async move {
        let _permit = permit;
        loop {
            let next =
                tokio::select! { _ = stopped.changed() => break, next = incoming.recv() => next };
            let Some(Next {
                deadline,
                mut response,
            }) = next
            else {
                break;
            };
            let reading = async {
                let result = client.next_request().send().promise.await?;
                yolo_capnp_codec::read_value(result.get()?.get_event()?).map_err(Error::from)
            };
            let reading = async {
                match deadline {
                    Some(deadline) => timeout_at(deadline, reading)
                        .await
                        .map_err(|_| Error::Deadline)?,
                    None => reading.await,
                }
            };
            let result = tokio::select! {
                _ = stopped.changed() => break,
                _ = response.closed() => break,
                result = reading => result,
            };
            let failed = result.is_err();
            if response.send(result).is_err() || failed {
                break;
            }
        }
        // Explicitly relinquish the source capability; no terminal lifecycle command is sent.
        let cancel = client.cancel_request().send();
        let _ = timeout_at(
            Instant::now() + crate::engine::ADMISSION_TIMEOUT,
            cancel.promise,
        )
        .await;
    });
    sender
}
