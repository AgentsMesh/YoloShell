use crate::{constants::*, error, handler::Handler, socket::SocketOwner};
use std::io::ErrorKind;
use tokio::{
    sync::Mutex,
    time::{Instant, timeout_at},
};
use yolo_hub_client::{ClientError, HubEntryStream};
use yolo_hub_client_transport::ClientConnection;

pub(crate) struct Connection {
    pub inner: ClientConnection,
    pub outer: Mutex<Option<russh::client::Handle<Handler>>>,
    pub owner: SocketOwner,
}
impl Connection {
    pub fn is_closed(&self) -> bool {
        self.owner.0.is_closed() || self.inner.is_closed()
    }
    pub fn close_now(&self) {
        self.owner.0.close();
    }
    pub async fn open(&self, deadline: Instant) -> Result<HubEntryStream, ClientError> {
        let stream =
            self.inner.open_channel(deadline).await.map_err(|_| {
                error::stage(deadline, ErrorKind::ConnectionAborted, CHANNEL_FAILED)
            })?;
        Ok(HubEntryStream::Remote(Box::new(stream)))
    }
    pub async fn close(&self, deadline: Instant) -> Result<(), ClientError> {
        self.close_now();
        self.inner
            .close(deadline)
            .await
            .map_err(|_| error::stage(deadline, ErrorKind::ConnectionAborted, ENTRY_CLOSED))?;
        if let Some(outer) = timeout_at(deadline, self.outer.lock())
            .await
            .map_err(error::timeout)?
            .take()
        {
            let _ = timeout_at(deadline, outer).await.map_err(error::timeout)?;
        }
        Ok(())
    }
}
