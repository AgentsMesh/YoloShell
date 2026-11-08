use super::*;
use std::sync::Mutex;
use tokio::sync::{Notify, mpsc};
use yolo_capnp_rpc::EventSource;

pub const RESPONSE: &[u8] = b"\0\xffruntime";
pub const FOREIGN_ID: &str = "another-business-operation";
pub const EVENT_NAME: &str = "test.observation";
const QUEUE: usize = 4;

#[derive(Clone, Copy)]
pub enum Shape {
    Correlated,
    WrongId,
    Malformed,
    MissingPayload,
    UnknownMetadata,
}
pub struct Harness {
    pub client: RuntimeConnection,
    pub confirmed: Arc<AtomicUsize>,
    pub calls: Arc<AtomicUsize>,
    pub events: mpsc::Sender<Value>,
    pub entered: Arc<Notify>,
    pub retired: Arc<AtomicUsize>,
    serving: tokio::task::JoinHandle<Result<(), Error>>,
}
struct Endpoint {
    shape: Shape,
    confirmed: Arc<AtomicUsize>,
    calls: Arc<AtomicUsize>,
    events: Mutex<Option<Queue>>,
    receipt: bool,
}
struct Queue {
    incoming: mpsc::Receiver<Value>,
    entered: Arc<Notify>,
    retired: Arc<AtomicUsize>,
}
impl Drop for Queue {
    fn drop(&mut self) {
        self.retired.fetch_add(1, Ordering::SeqCst);
    }
}
impl EventSource for Queue {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async {
            self.entered.notify_one();
            self.incoming.recv().await.ok_or(Error::Closed)
        })
    }
}
impl Service for Endpoint {
    fn call(&self, _: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let request: RequestEnvelope = yolo_capnp_codec::from_value(value)?;
            let mut response = ResponseEnvelope::success(&request, &Value::Data(RESPONSE.to_vec()))
                .map_err(|_| Error::Invalid)?;
            if matches!(self.shape, Shape::WrongId) {
                response.request_id = FOREIGN_ID.into();
            }
            let mut value = if matches!(self.shape, Shape::Malformed) {
                Value::Boolean(false)
            } else {
                yolo_capnp_codec::to_value(&response)?
            };
            if let Value::Map(fields) = &mut value {
                match self.shape {
                    Shape::MissingPayload => {
                        fields.retain(|(key, _)| key.as_str() != Some(yolo_wire::field::PAYLOAD));
                    }
                    Shape::UnknownMetadata => {
                        fields.push((Value::from(yolo_wire::field::ROWS), Value::Null))
                    }
                    _ => {}
                }
            }
            let confirmed = self.confirmed.clone();
            let observed: Option<Box<dyn Confirmation>> = self.receipt.then(|| {
                Box::new(move || {
                    confirmed.fetch_add(1, Ordering::SeqCst);
                }) as Box<dyn Confirmation>
            });
            Ok(Reply {
                value,
                on_observed: observed,
                events: self
                    .events
                    .lock()
                    .unwrap()
                    .take()
                    .map(|events| Box::new(events) as Box<dyn EventSource>),
            })
        })
    }
}
impl Harness {
    pub async fn new(shape: Shape, with_events: bool, receipt: bool) -> Self {
        let (client, server) = tokio::io::duplex(PIPE_BYTES);
        let (events, incoming) = mpsc::channel(QUEUE);
        let confirmed = Arc::new(AtomicUsize::new(0));
        let calls = Arc::new(AtomicUsize::new(0));
        let entered = Arc::new(Notify::new());
        let retired = Arc::new(AtomicUsize::new(0));
        let queue = with_events.then(|| Queue {
            incoming,
            entered: entered.clone(),
            retired: retired.clone(),
        });
        let serving = tokio::spawn(yolo_capnp_rpc::serve(
            server,
            Protocol::Endpoint,
            Arc::new(Endpoint {
                shape,
                confirmed: confirmed.clone(),
                calls: calls.clone(),
                events: Mutex::new(queue),
                receipt,
            }),
        ));
        Self {
            client: RuntimeConnection::open(client, deadline()).await.unwrap(),
            confirmed,
            calls,
            events,
            entered,
            retired,
            serving,
        }
    }
    pub async fn finish(mut self) {
        self.client.close();
        drop(self.client);
        tokio::time::timeout(DEADLINE, self.serving)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
    }
}
pub fn request() -> RequestEnvelope {
    RequestEnvelope::new(REQUEST_ID, METHOD, Value::Null)
}
pub fn deadline() -> Instant {
    Instant::now() + DEADLINE
}
pub async fn wait_for_retirement(retired: &AtomicUsize) {
    tokio::time::timeout(DEADLINE, async {
        while retired.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}
