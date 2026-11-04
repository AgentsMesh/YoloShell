use super::super::super::constants::RUNTIME_SERVER_NAME;
use super::*;
use crate::host::HelloResult;
use crate::model::OutputChunk;
use crate::protocol::{EventEnvelope, RequestEnvelope, ResponseEnvelope, method};
use crate::worker::{AttachRequest, AttachResult};
use yolo_capnp_rpc::{Error, EventSource, Future, Method, Protocol, Reply, Service, Value};

pub(super) async fn serve_runtime(
    stream: tokio::io::DuplexStream,
    role: RuntimeRole,
    status: HostStatus,
    break_control_after_handshake: bool,
    behavior: RuntimeBehavior,
    requests: Arc<Mutex<Vec<String>>>,
) {
    let service = Server {
        role,
        status,
        break_control_after_handshake,
        behavior,
        responses: AtomicU64::new(0),
        requests,
    };
    let _ = yolo_capnp_rpc::serve(stream, Protocol::Endpoint, Arc::new(service)).await;
}
struct Server {
    role: RuntimeRole,
    status: HostStatus,
    break_control_after_handshake: bool,
    behavior: RuntimeBehavior,
    responses: AtomicU64,
    requests: Arc<Mutex<Vec<String>>>,
}
impl Service for Server {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            self.requests.lock().unwrap().push(request.method.clone());
            let count = self.responses.fetch_add(1, Ordering::SeqCst);
            if self.role == RuntimeRole::Attachment && request.method == method::ATTACH_SESSION {
                match &self.behavior {
                    RuntimeBehavior::StallAttachment(reached)
                    | RuntimeBehavior::DelayOpenThenStallAttachment { reached, .. } => {
                        reached.notify_one();
                        std::future::pending::<()>().await;
                    }
                    _ => {}
                }
                return attachment(&request);
            }
            if count == 0 {
                match &self.behavior {
                    RuntimeBehavior::StallHello(reached) => {
                        reached.notify_one();
                        std::future::pending::<()>().await;
                    }
                    RuntimeBehavior::DelayHelloThenStallStatus { delay, started, .. } => {
                        started.notify_one();
                        tokio::time::sleep(*delay).await;
                    }
                    _ => {}
                }
            }
            if count == 1 {
                match &self.behavior {
                    RuntimeBehavior::StallStatus(reached)
                    | RuntimeBehavior::DelayHelloThenStallStatus { reached, .. } => {
                        reached.notify_one();
                        std::future::pending::<()>().await;
                    }
                    _ => {}
                }
            }
            if self.break_control_after_handshake && count >= 2 {
                return Err(Error::Closed);
            }
            let response = if request.method == method::HELLO {
                ResponseEnvelope::success(
                    &request,
                    &HelloResult {
                        server_name: RUNTIME_SERVER_NAME.into(),
                        protocol_version: crate::model::PROTOCOL_VERSION,
                        capabilities: self.status.capabilities.clone(),
                    },
                )
                .unwrap()
            } else if request.method == method::HOST_STATUS {
                ResponseEnvelope::success(&request, &self.status).unwrap()
            } else {
                ResponseEnvelope::empty(&request)
            };
            Ok(Reply::value(yolo_capnp_codec::to_value(&response)?))
        })
    }
}
fn attachment(request: &RequestEnvelope) -> Result<Reply, Error> {
    let attach: AttachRequest = request.decode_payload().map_err(|_| Error::Invalid)?;
    let runtime_epoch = attach.terminal_id;
    let result = AttachResult {
        runtime_epoch,
        snapshot: None,
        chunks: vec![],
        next_sequence: TEST_ATTACHMENT_NEXT_SEQUENCE,
        resynchronized: false,
    };
    let response = ResponseEnvelope::success(request, &result).map_err(|_| Error::Invalid)?;
    let event = EventEnvelope::new(
        crate::protocol::event::TERMINAL_OUTPUT,
        Some(attach.terminal_id.to_string()),
        Some(runtime_epoch.to_string()),
        &OutputChunk {
            runtime_epoch,
            sequence: result.next_sequence,
            data: Vec::new(),
        },
    )
    .map_err(|_| Error::Invalid)?;
    Ok(Reply {
        value: yolo_capnp_codec::to_value(&response)?,
        events: Some(Box::new(Events(Some(event)))),
        on_observed: None,
    })
}
struct Events(Option<EventEnvelope>);
impl EventSource for Events {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async {
            yolo_capnp_codec::to_value(&self.0.take().ok_or(Error::Closed)?).map_err(Error::from)
        })
    }
}
