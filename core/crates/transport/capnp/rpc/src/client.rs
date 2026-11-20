use crate::{Value, api::*, connection::Call, constants::*, engine, event_client};
use capnp_rpc::{RpcSystem, rpc_twoparty_capnp::Side, twoparty};
use std::sync::Arc;
use tokio::{
    sync::{Semaphore, mpsc, watch},
    task::JoinSet,
    time::{Instant, timeout_at},
};
use tokio_util::compat::{TokioAsyncReadCompatExt, TokioAsyncWriteCompatExt};
use yolo_capnp_schema::rpc_capnp::{endpoint, peer, result};

pub(crate) struct LocalReply {
    pub value: Value,
    pub events: Option<mpsc::Sender<event_client::Next>>,
    pub receipt: Option<tokio::sync::oneshot::Sender<crate::receipt::ConfirmRequest>>,
}
#[derive(Clone)]
enum Bootstrap {
    Endpoint(endpoint::Client),
    Peer(peer::Client),
}

pub(crate) async fn run(
    io: Box<dyn Duplex>,
    protocol: Protocol,
    mut calls: mpsc::Receiver<Call>,
    mut stopped: watch::Receiver<bool>,
) {
    let (read, write) = tokio::io::split(io);
    let network = twoparty::VatNetwork::new(
        read.compat(),
        write.compat_write(),
        Side::Client,
        engine::options(),
    );
    let mut system = RpcSystem::new(Box::new(network), None);
    let client = match protocol {
        Protocol::Endpoint => Bootstrap::Endpoint(system.bootstrap(Side::Server)),
        Protocol::Peer => Bootstrap::Peer(system.bootstrap(Side::Server)),
    };
    let slots = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
    let streams = Arc::new(Semaphore::new(MAX_STREAMS));
    let (ended, finished) = watch::channel(false);
    let _completion = Completion(ended);
    let mut tasks = JoinSet::new();
    let mut peer_closed = false;
    tokio::pin!(system);
    loop {
        tokio::select! {
            _ = stopped.changed() => break,
            _ = &mut system => { peer_closed = true; break; },
            _ = tasks.join_next(), if !tasks.is_empty() => {},
            call = calls.recv() => {
                let Some(call) = call else { break; };
                let Ok(permit) = slots.clone().try_acquire_owned() else { let _ = call.response.send(Err(Error::Capacity)); continue; };
                let client = client.clone(); let streams = streams.clone(); let stop = finished.clone();
                tasks.spawn_local(async move {
                    let _permit = permit;
                    let Call { method, value, deadline, deferred, mut response } = call;
                    let result = tokio::select! {
                        _ = response.closed() => return,
                        result = invoke(client, method, value, streams, stop, deadline, deferred) => result,
                    };
                    let _ = response.send(result);
                });
            }
        }
    }
    calls.close();
    // A decoded response stays observed even if the peer closes before its confirmation reply.
    if !peer_closed {
        tasks.abort_all();
    }
    while tasks.join_next().await.is_some() {}
}

struct Completion(watch::Sender<bool>);
impl Drop for Completion {
    fn drop(&mut self) {
        self.0.send_replace(true);
    }
}

async fn invoke(
    client: Bootstrap,
    method: Method,
    value: Value,
    slots: Arc<Semaphore>,
    stopped: watch::Receiver<bool>,
    deadline: Instant,
    deferred: bool,
) -> Result<LocalReply, Error> {
    match (client, method) {
        (Bootstrap::Endpoint(client), Method::Call) => {
            let mut request = client.call_request();
            yolo_capnp_codec::write_value(request.get().init_request(), &value)?;
            require_dispatch_budget(deadline)?;
            let response = timeout_at(deadline, request.send().promise)
                .await
                .map_err(|_| Error::OutcomeUnknown)?
                .map_err(|_| Error::OutcomeUnknown)?;
            decoded(
                response
                    .get()
                    .and_then(|value| value.get_result())
                    .map_err(|_| Error::OutcomeUnknown)?,
                slots,
                stopped,
                deadline,
                deferred,
            )
            .await
            .map_err(|_| Error::OutcomeUnknown)
        }
        (Bootstrap::Peer(client), Method::Discover) => {
            let mut request = client.discover_request();
            yolo_capnp_codec::write_value(request.get().init_request(), &value)?;
            require_dispatch_budget(deadline)?;
            let response = timeout_at(deadline, request.send().promise)
                .await
                .map_err(|_| Error::Deadline)??;
            decoded(
                response.get()?.get_result()?,
                slots,
                stopped,
                deadline,
                deferred,
            )
            .await
        }
        (Bootstrap::Peer(client), Method::Execute) => {
            let mut request = client.execute_request();
            yolo_capnp_codec::write_value(request.get().init_request(), &value)?;
            require_dispatch_budget(deadline)?;
            let response = timeout_at(deadline, request.send().promise)
                .await
                .map_err(|_| Error::OutcomeUnknown)?
                .map_err(|_| Error::OutcomeUnknown)?;
            decoded(
                response
                    .get()
                    .and_then(|value| value.get_result())
                    .map_err(|_| Error::OutcomeUnknown)?,
                slots,
                stopped,
                deadline,
                deferred,
            )
            .await
            .map_err(|_| Error::OutcomeUnknown)
        }
        _ => Err(Error::Invalid),
    }
}
fn require_dispatch_budget(deadline: Instant) -> Result<(), Error> {
    if Instant::now() >= deadline {
        Err(Error::Deadline)
    } else {
        Ok(())
    }
}

async fn decoded(
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
