//! Peer discovery, execution and signed terminal capabilities over real Cap'n Proto RPC.
use std::sync::Arc;
use tokio::time::Instant;
use yolo_capnp_rpc::{
    Connection, Duplex, Error, EventSource, Future, Method, Protocol, Service, Value,
};
pub use yolo_capnp_rpc::{Error as RpcError, Future as RpcFuture};
use yolo_hub_peer_protocol::{PeerReply, PeerRequest, Signed, TerminalUpdate};

pub trait Handler: Send + Sync + 'static {
    fn request(&self, request: PeerRequest) -> Future<'_, Response>;
}
pub trait Updates: Send + 'static {
    fn next(&mut self) -> Future<'_, Signed<TerminalUpdate>>;
}
pub struct Response {
    pub reply: PeerReply,
    pub updates: Option<Box<dyn Updates>>,
}
pub struct Received {
    pub reply: PeerReply,
    pub updates: Option<RemoteUpdates>,
}
pub struct Client {
    connection: Connection,
}
pub struct RemoteUpdates {
    events: yolo_capnp_rpc::RemoteEvents,
}

impl Client {
    pub async fn connect(stream: impl Duplex + 'static, deadline: Instant) -> Result<Self, Error> {
        Ok(Self {
            connection: Connection::open(stream, Protocol::Peer, deadline).await?,
        })
    }
    pub async fn request(
        &self,
        request: PeerRequest,
        deadline: Instant,
    ) -> Result<Received, Error> {
        let method = match &request {
            PeerRequest::Discover { .. } => Method::Discover,
            PeerRequest::Execute { .. } => Method::Execute,
        };
        let response = self
            .connection
            .request(method, yolo_capnp_codec::to_value(&request)?, deadline)
            .await?;
        Ok(Received {
            reply: yolo_capnp_codec::from_value(response.value)?,
            updates: response.events.map(|events| RemoteUpdates { events }),
        })
    }
}
impl RemoteUpdates {
    pub async fn next(&mut self, deadline: Instant) -> Result<Signed<TerminalUpdate>, Error> {
        yolo_capnp_codec::from_value(self.events.next(deadline).await?).map_err(Error::from)
    }
}

pub async fn serve(stream: impl Duplex + 'static, handler: Arc<dyn Handler>) -> Result<(), Error> {
    yolo_capnp_rpc::serve(stream, Protocol::Peer, Arc::new(Adapter(handler))).await
}
struct Adapter(Arc<dyn Handler>);
impl Service for Adapter {
    fn call(&self, method: Method, value: Value) -> Future<'_, yolo_capnp_rpc::Reply> {
        Box::pin(async move {
            let request: PeerRequest = yolo_capnp_codec::from_value(value)?;
            if !matches!(
                (method, &request),
                (Method::Discover, PeerRequest::Discover { .. })
                    | (Method::Execute, PeerRequest::Execute { .. })
            ) {
                return Err(Error::Invalid);
            }
            let response = self.0.request(request).await?;
            Ok(yolo_capnp_rpc::Reply {
                value: yolo_capnp_codec::to_value(&response.reply)?,
                events: response
                    .updates
                    .map(|source| Box::new(Source(source)) as Box<dyn EventSource>),
                on_observed: None,
            })
        })
    }
}
struct Source(Box<dyn Updates>);
impl EventSource for Source {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async { yolo_capnp_codec::to_value(&self.0.next().await?).map_err(Error::from) })
    }
}
