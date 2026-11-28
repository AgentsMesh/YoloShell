use super::{LocalReply, event_client};
use crate::api::Error;
use std::sync::Arc;
use tokio::{
    sync::{Semaphore, watch},
    time::Instant,
};
use yolo_capnp_schema::rpc_capnp::result;

pub(super) async fn decoded(
    result: result::Reader<'_>,
    slots: Arc<Semaphore>,
    stopped: watch::Receiver<bool>,
    deadline: Instant,
    deferred: bool,
) -> Result<LocalReply, Error> {
    let value = yolo_capnp_codec::read_value(result.get_value()?)?;
    let events = if result.has_events() {
        let permit = slots.try_acquire_owned().map_err(|_| Error::Capacity)?;
        Some(event_client::spawn(
            result.get_events()?,
            stopped.clone(),
            permit,
        ))
    } else {
        None
    };
    let receipt = if deferred && result.has_receipt() {
        Some(crate::receipt::defer(result.get_receipt()?, stopped))
    } else {
        crate::receipt::confirm(result, deadline).await?;
        None
    };
    Ok(LocalReply {
        value,
        events,
        receipt,
    })
}
