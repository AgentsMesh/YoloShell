//! Scripted test peer backed by the production Cap'n Proto capability server.
use std::sync::Arc;
use tokio::{
    sync::{mpsc, oneshot},
    task::JoinHandle,
};
use yolo_capnp_rpc::{Duplex, Error, EventSource, Future, Method, Protocol, Reply, Service, Value};
use yolo_wire::{EventEnvelope, ProtocolError, RequestEnvelope, ResponseEnvelope};

const SCRIPT_CAPACITY: usize = 16;
const SCHEDULING_BUDGET: std::time::Duration = std::time::Duration::from_secs(3);

/// Keep the test runtime runnable while an independent RPC executor schedules;
/// this prevents Tokio's paused clock auto-advancing before a request arrives.
pub async fn scheduled<T>(mut receiver: oneshot::Receiver<T>) -> T {
    let deadline = std::time::Instant::now() + SCHEDULING_BUDGET;
    loop {
        match receiver.try_recv() {
            Ok(value) => return value,
            Err(oneshot::error::TryRecvError::Closed) => panic!("RPC schedule signal closed"),
            Err(oneshot::error::TryRecvError::Empty) => {}
        }
        assert!(
            std::time::Instant::now() < deadline,
            "RPC scheduling deadline exceeded"
        );
        tokio::task::yield_now().await;
    }
}

struct Invocation {
    request: RequestEnvelope,
    result: oneshot::Sender<Reply>,
}
struct Script {
    incoming: mpsc::Sender<Invocation>,
}
impl Service for Script {
    fn call(&self, method: Method, value: Value) -> Future<'_, Reply> {
        Box::pin(async move {
            if method != Method::Call {
                return Err(Error::Invalid);
            }
            let request = yolo_capnp_codec::from_value(value)?;
            let (result, response) = oneshot::channel();
            self.incoming
                .send(Invocation { request, result })
                .await
                .map_err(|_| Error::Closed)?;
            response.await.map_err(|_| Error::Closed)
        })
    }
}
struct Events(mpsc::Receiver<Value>);
impl EventSource for Events {
    fn next(&mut self) -> Future<'_, Value> {
        Box::pin(async move { self.0.recv().await.ok_or(Error::Closed) })
    }
}

pub struct Peer {
    incoming: mpsc::Receiver<Invocation>,
    pending: Option<oneshot::Sender<Reply>>,
    task: JoinHandle<Result<(), Error>>,
    event_sender: Option<mpsc::Sender<Value>>,
    event_receiver: Option<mpsc::Receiver<Value>>,
    attach: bool,
}

impl Peer {
    pub fn new(stream: impl Duplex + 'static) -> Self {
        let (sender, incoming) = mpsc::channel(SCRIPT_CAPACITY);
        let (event_sender, event_receiver) = mpsc::channel(SCRIPT_CAPACITY);
        let task = tokio::spawn(yolo_capnp_rpc::serve(
            stream,
            Protocol::Endpoint,
            Arc::new(Script { incoming: sender }),
        ));
        Self {
            incoming,
            pending: None,
            task,
            event_sender: Some(event_sender),
            event_receiver: Some(event_receiver),
            attach: false,
        }
    }
    pub async fn request(&mut self) -> Result<RequestEnvelope, ProtocolError> {
        let invocation = tokio::select! {
            invocation = self.incoming.recv() => invocation.ok_or(ProtocolError::EndOfStream)?,
            _ = &mut self.task => return Err(ProtocolError::EndOfStream),
        };
        self.attach = invocation.request.method == yolo_wire::method::ATTACH_SESSION;
        assert!(self.pending.replace(invocation.result).is_none());
        Ok(invocation.request)
    }
    pub async fn respond(&mut self, response: &ResponseEnvelope) -> Result<(), ProtocolError> {
        let (observed, receipt) = oneshot::channel();
        let events = if self.attach {
            self.event_receiver
                .take()
                .map(|events| Box::new(Events(events)) as Box<dyn EventSource>)
        } else {
            None
        };
        let reply = Reply {
            value: yolo_capnp_codec::to_value(response).map_err(ProtocolError::Encode)?,
            events,
            on_observed: Some(Box::new(move || {
                let _ = observed.send(());
            })),
        };
        self.pending
            .take()
            .ok_or(ProtocolError::ConnectionPoisoned)?
            .send(reply)
            .map_err(|_| ProtocolError::EndOfStream)?;
        receipt.await.map_err(|_| ProtocolError::EndOfStream)
    }
    pub async fn event(&self, event: &EventEnvelope) -> Result<(), ProtocolError> {
        self.raw_event(yolo_capnp_codec::to_value(event).map_err(ProtocolError::Encode)?)
            .await
    }
    pub async fn raw_event(&self, value: Value) -> Result<(), ProtocolError> {
        self.event_sender
            .as_ref()
            .ok_or(ProtocolError::EndOfStream)?
            .send(value)
            .await
            .map_err(|_| ProtocolError::EndOfStream)
    }
    pub fn finish_events(&mut self) {
        self.event_sender.take();
    }
    pub async fn close(&mut self) {
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}

impl Drop for Peer {
    fn drop(&mut self) {
        self.task.abort();
    }
}
