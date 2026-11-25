use super::model::*;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};
use yolo_hub_dispatch::{Dispatcher, Operation, OperationFuture};
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Reply, Request};
use yolo_hub_terminal_protocol::PublicError;

pub struct Scripted {
    pub origin: HubId,
    pub calls: Mutex<Vec<Request>>,
    replies: Mutex<VecDeque<Reply>>,
}
impl Scripted {
    pub fn new(replies: impl IntoIterator<Item = Reply>) -> Arc<Self> {
        Arc::new(Self {
            origin: hub(ENTRY),
            calls: Mutex::default(),
            replies: Mutex::new(replies.into_iter().collect()),
        })
    }
}
impl Dispatcher for Scripted {
    fn hub_id(&self) -> HubId {
        self.origin
    }
    fn dispatch(&self, request: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            self.calls.lock().unwrap().push(request);
            self.replies
                .lock()
                .unwrap()
                .pop_front()
                .map(Operation::Reply)
                .ok_or(PublicError::Internal)
        })
    }
}

pub fn reply(operation: Result<Operation, PublicError>) -> Reply {
    match operation.unwrap() {
        Operation::Reply(reply) => reply,
        Operation::Attached(_) => panic!("unexpected attachment"),
    }
}

pub struct Blocking {
    pub started: tokio::sync::Notify,
    pub released: tokio::sync::Notify,
    pub dispatched: std::sync::atomic::AtomicBool,
}
impl Default for Blocking {
    fn default() -> Self {
        Self {
            started: Default::default(),
            released: Default::default(),
            dispatched: Default::default(),
        }
    }
}
impl Dispatcher for Blocking {
    fn hub_id(&self) -> HubId {
        hub(ENTRY)
    }
    fn dispatch(&self, _: Request) -> OperationFuture<'_> {
        Box::pin(async move {
            self.started.notify_one();
            self.released.notified().await;
            self.dispatched
                .store(true, std::sync::atomic::Ordering::Release);
            Ok(Operation::Reply(Reply::Completed {}))
        })
    }
}
