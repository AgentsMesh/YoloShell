use crate::{Value, api::*, constants::*, engine, event_client::RemoteEvents};
use std::sync::Arc;
use tokio::{
    sync::{mpsc, oneshot, watch},
    time::{Instant, timeout_at},
};

pub(crate) struct Call {
    pub method: Method,
    pub value: Value,
    pub deadline: Instant,
    pub deferred: bool,
    pub response: oneshot::Sender<Result<crate::client::LocalReply, Error>>,
}
pub(crate) struct Inner {
    pub calls: mpsc::Sender<Call>,
    pub stop: watch::Sender<bool>,
}
impl Drop for Inner {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
#[derive(Clone)]
pub struct Connection(pub(crate) Arc<Inner>);
pub struct RemoteReply {
    pub value: Value,
    pub events: Option<RemoteEvents>,
    pub receipt: Option<crate::receipt::DeferredReceipt>,
}

impl Connection {
    pub async fn open(
        io: impl Duplex + 'static,
        protocol: Protocol,
        deadline: Instant,
    ) -> Result<Self, Error> {
        let (sender, calls) = mpsc::channel(MAX_IN_FLIGHT);
        let (stop, stopped) = watch::channel(false);
        let connection = Self(Arc::new(Inner {
            calls: sender,
            stop,
        }));
        engine::submit(
            engine::Job::Client {
                io: Box::new(io),
                protocol,
                calls,
                stopped,
            },
            deadline,
        )
        .await?;
        Ok(connection)
    }
    /// Expiration cancels this invocation. Nothing retries or replays its business operation.
    pub async fn request(
        &self,
        method: Method,
        value: Value,
        deadline: Instant,
    ) -> Result<RemoteReply, Error> {
        self.request_mode(method, value, deadline, false).await
    }
    pub async fn request_deferred(
        &self,
        method: Method,
        value: Value,
        deadline: Instant,
    ) -> Result<RemoteReply, Error> {
        self.request_mode(method, value, deadline, true).await
    }
    async fn request_mode(
        &self,
        method: Method,
        value: Value,
        deadline: Instant,
        deferred: bool,
    ) -> Result<RemoteReply, Error> {
        if *self.0.stop.borrow() {
            return Err(Error::Closed);
        }
        if Instant::now() >= deadline {
            return Err(Error::Deadline);
        }
        let (response, result) = oneshot::channel();
        timeout_at(
            deadline,
            self.0.calls.send(Call {
                method,
                value,
                deadline,
                deferred,
                response,
            }),
        )
        .await
        .map_err(|_| Error::Deadline)?
        .map_err(|_| Error::Closed)?;
        // Once admitted to the executor, loss of the invocation task cannot
        // prove that a business call was never transmitted or committed.
        let reply = timeout_at(deadline, result)
            .await
            .map_err(|_| match method {
                Method::Call | Method::Execute => Error::OutcomeUnknown,
                Method::Discover => Error::Deadline,
            })?
            .map_err(|_| match method {
                Method::Call | Method::Execute => Error::OutcomeUnknown,
                Method::Discover => Error::Closed,
            })??;
        Ok(RemoteReply {
            value: reply.value,
            receipt: reply.receipt.map(|sender| crate::receipt::DeferredReceipt {
                sender,
                connection: self.clone(),
            }),
            events: reply.events.map(|sender| RemoteEvents {
                sender,
                _connection: self.clone(),
            }),
        })
    }
    pub fn close(&self) {
        self.0.stop.send_replace(true);
    }
    pub fn is_closed(&self) -> bool {
        *self.0.stop.borrow()
    }
}

impl RemoteReply {
    /// Proxies preserve publication custody until the downstream observer confirms this reply.
    pub fn into_service_reply(self) -> Reply {
        Reply {
            value: self.value,
            events: self
                .events
                .map(|events| Box::new(events) as Box<dyn EventSource>),
            on_observed: self
                .receipt
                .map(|receipt| Box::new(receipt) as Box<dyn Confirmation>),
        }
    }
}
impl EventSource for RemoteEvents {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(self.next_unbounded())
    }
}

pub async fn serve(
    io: impl Duplex + 'static,
    protocol: Protocol,
    service: Arc<dyn Service>,
) -> Result<(), Error> {
    let (stop, stopped) = watch::channel(false);
    let _guard = ServerGuard {
        stop,
        _service: service.clone(),
    };
    let (completion, done) = oneshot::channel();
    engine::submit(
        engine::Job::Server {
            io: Box::new(io),
            protocol,
            service: Arc::downgrade(&service),
            stopped,
            completion,
            domain: tokio::runtime::Handle::current(),
        },
        Instant::now() + engine::ADMISSION_TIMEOUT,
    )
    .await?;
    done.await.map_err(|_| Error::Closed)?
}
struct ServerGuard {
    stop: watch::Sender<bool>,
    _service: Arc<dyn Service>,
}
impl Drop for ServerGuard {
    fn drop(&mut self) {
        self.stop.send_replace(true);
    }
}
