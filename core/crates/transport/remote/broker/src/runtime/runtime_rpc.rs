use super::BrokerConnectionPool;
use super::constants::RUNTIME_ATTACHMENT_LIMIT_MESSAGE;
use crate::{
    protocol::{RequestEnvelope, method},
    remote::{RemoteDeadline, RuntimeRole},
};
use std::sync::Arc;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use yolo_capnp_rpc::{Error, EventSource, Future, Method, Reply, Service, Value};

pub(super) struct RuntimeProxy {
    pub pool: Arc<BrokerConnectionPool>,
    pub attachments: Option<Arc<Semaphore>>,
    pub control: Mutex<Option<yolo_runtime_rpc::RuntimeConnection>>,
    pub admission: super::runtime_admission::RuntimeAdmission,
}
impl Service for RuntimeProxy {
    fn call(&self, rpc_method: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            if rpc_method != Method::Call {
                return Err(Error::Invalid);
            }
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            self.admission.started();
            let deadline = RemoteDeadline::operation();
            if request.method == method::ATTACH_SESSION {
                let permit = match &self.attachments {
                    Some(attachments) => match crate::ingress::try_admit(attachments) {
                        Some(permit) => Some(permit),
                        None => {
                            // Admission failed before opening or forwarding to the remote runtime.
                            let response = crate::protocol::ResponseEnvelope::failure(
                                &request,
                                crate::protocol::ErrorBody::conflict(
                                    RUNTIME_ATTACHMENT_LIMIT_MESSAGE,
                                ),
                            );
                            return Ok(Reply::value(yolo_capnp_codec::to_value(&response)?));
                        }
                    },
                    None => None,
                };
                let mut connection = self
                    .pool
                    .open_runtime(RuntimeRole::Attachment)
                    .await
                    .map_err(remote_error)?;
                self.admission.wait().await?;
                let response = connection
                    .forward(&request, deadline.instant())
                    .await
                    .map_err(runtime_error)?;
                let mut reply = response.into_service_reply();
                if let Some(events) = reply.events.take() {
                    reply.events = Some(Box::new(Attachment {
                        events,
                        _permit: permit,
                    }));
                }
                return Ok(reply);
            }
            let mut retained = self.control.lock().await;
            if retained.is_none() {
                *retained = Some(
                    self.pool
                        .open_runtime(RuntimeRole::Control)
                        .await
                        .map_err(remote_error)?,
                );
            }
            let connection = retained.as_mut().ok_or(Error::Closed)?;
            self.admission.wait().await?;
            match connection.forward(&request, deadline.instant()).await {
                Ok(reply) => Ok(reply.into_service_reply()),
                Err(cause) => {
                    retained.take();
                    Err(runtime_error(cause))
                }
            }
        })
    }
}
struct Attachment {
    events: Box<dyn EventSource>,
    _permit: Option<OwnedSemaphorePermit>,
}
impl EventSource for Attachment {
    fn next(&mut self) -> Future<'_, Value> {
        self.events.next()
    }
}
fn runtime_error(error: crate::protocol::ProtocolError) -> Error {
    match error {
        crate::protocol::ProtocolError::OutcomeUnknown => Error::OutcomeUnknown,
        other => Error::Remote(other.to_string()),
    }
}
fn remote_error(error: crate::remote::RemoteError) -> Error {
    match error {
        crate::remote::RemoteError::Protocol(error) => runtime_error(error),
        other => Error::Remote(other.to_string()),
    }
}
