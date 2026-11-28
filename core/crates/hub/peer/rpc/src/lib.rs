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
    fn watch_directory(&self, _request: PeerRequest) -> Future<'_, Response> {
        Box::pin(async { Err(Error::Unimplemented) })
    }
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
#[derive(Clone)]
pub struct Client {
    connection: Connection,
    can_watch: Arc<std::sync::atomic::AtomicBool>,
    renewal_deadline: Arc<std::sync::atomic::AtomicU64>,
}
pub struct RemoteUpdates {
    events: yolo_capnp_rpc::RemoteEvents,
}

impl Client {
    pub async fn connect(stream: impl Duplex + 'static, deadline: Instant) -> Result<Self, Error> {
        Ok(Self {
            connection: Connection::open(stream, Protocol::Peer, deadline).await?,
            can_watch: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            renewal_deadline: Arc::new(std::sync::atomic::AtomicU64::new(u64::MAX)),
        })
    }
    pub fn is_closed(&self) -> bool {
        self.connection.is_closed()
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
        self.send(method, &request, deadline).await
    }

    /// Old peers reject the appended Cap'n Proto method on the same connection.
    /// A transport or admission failure remains an error for the caller.
    pub async fn observe_directory(
        &self,
        request: PeerRequest,
        deadline: Instant,
        renew_by: u64,
    ) -> Result<Received, Error> {
        use std::sync::atomic::Ordering;
        if !matches!(request, PeerRequest::Discover { .. }) {
            return Err(Error::Invalid);
        }
        let result = if self.can_watch.load(Ordering::Acquire)
            && self.renewal_deadline.load(Ordering::Acquire) > renew_by
        {
            match self.send(Method::WatchDirectory, &request, deadline).await {
                Err(Error::Unimplemented) => {
                    self.can_watch.store(false, Ordering::Release);
                    self.send(Method::Discover, &request, deadline).await
                }
                result => result,
            }
        } else {
            self.send(Method::Discover, &request, deadline).await
        }?;
        if let PeerReply::Directory { origins, .. } = &result.reply {
            let expires = origins
                .iter()
                .flat_map(|origin| {
                    std::iter::once(origin.catalogue.payload.expires_at_ms).chain(
                        origin
                            .authority
                            .links()
                            .iter()
                            .map(|link| link.fields().claims.expires_at_ms),
                    )
                })
                .min()
                .unwrap_or(u64::MAX);
            self.renewal_deadline.store(expires, Ordering::Release);
        }
        Ok(result)
    }

    async fn send(
        &self,
        method: Method,
        request: &PeerRequest,
        deadline: Instant,
    ) -> Result<Received, Error> {
        let response = self
            .connection
            .request(method, yolo_capnp_codec::to_value(request)?, deadline)
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
                (
                    Method::Discover | Method::WatchDirectory,
                    PeerRequest::Discover { .. }
                ) | (Method::Execute, PeerRequest::Execute { .. })
            ) {
                return Err(Error::Invalid);
            }
            let response = if method == Method::WatchDirectory {
                self.0.watch_directory(request).await?
            } else {
                self.0.request(request).await?
            };
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
