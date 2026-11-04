use super::gate::DropGate;
use std::sync::Mutex;
use yolo_hub_peer_protocol::{PeerReply, PeerRequest, Signed, TerminalUpdate};
use yolo_hub_peer_rpc::{Handler, Response, RpcFuture, Updates};

pub struct UpdateHandler(pub Mutex<Option<Box<dyn Updates>>>);
impl Handler for UpdateHandler {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, Response> {
        Box::pin(async move {
            let PeerRequest::Discover { request_id, .. } = request else {
                return Err(yolo_hub_peer_rpc::RpcError::Invalid);
            };
            Ok(Response {
                reply: PeerReply::Directory {
                    request_id,
                    revision: yolo_hub_peer_protocol::FIRST_REVISION,
                    origins: Vec::new(),
                },
                updates: self.0.lock().unwrap().take(),
            })
        })
    }
}

pub struct PendingUpdates;
impl Updates for PendingUpdates {
    fn next(&mut self) -> RpcFuture<'_, Signed<TerminalUpdate>> {
        Box::pin(std::future::pending())
    }
}

pub struct DelayedBridge {
    pub _gate: DropGate,
    pub updates: Box<dyn Updates>,
}
impl Updates for DelayedBridge {
    fn next(&mut self) -> RpcFuture<'_, Signed<TerminalUpdate>> {
        self.updates.next()
    }
}
