use super::constants::*;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};
use tokio::{sync::Notify, time::Instant};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_peer_dialer::{PeerDialFuture, PeerDialer};
use yolo_hub_peer_transport::PeerIo;

#[derive(Clone, Copy)]
pub enum Behavior {
    Ready,
    Pending,
    Reject,
}

pub struct Injected {
    pub entered: Notify,
    pub calls: AtomicUsize,
    pub seen: Mutex<Vec<(PeerIdentity, PeerAddress, Instant)>>,
    io: Mutex<Option<PeerIo>>,
    behavior: Behavior,
}

impl Injected {
    pub fn new(io: Option<PeerIo>, behavior: Behavior) -> Self {
        Self {
            entered: Notify::new(),
            calls: AtomicUsize::default(),
            seen: Mutex::new(Vec::new()),
            io: Mutex::new(io),
            behavior,
        }
    }
}
impl PeerDialer for Injected {
    fn dial<'a>(
        &'a self,
        peer: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async move {
            self.calls.fetch_add(ONE, Ordering::SeqCst);
            self.seen
                .lock()
                .unwrap()
                .push((peer, address.clone(), deadline));
            let transport = self.io.lock().unwrap().take();
            self.entered.notify_one();
            match self.behavior {
                Behavior::Ready => Ok(transport.unwrap()),
                Behavior::Reject => anyhow::bail!(REJECTED),
                Behavior::Pending => {
                    std::future::pending::<()>().await;
                    Ok(transport.unwrap())
                }
            }
        })
    }
}
