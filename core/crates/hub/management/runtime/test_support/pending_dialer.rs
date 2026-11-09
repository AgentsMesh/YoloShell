use super::constants::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::{sync::mpsc, time::Instant};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::PeerAddress;
use yolo_hub_peer_dialer::{PeerDialFuture, PeerDialer};
use yolo_hub_peer_transport::PeerIo;

pub enum Event {
    Started {
        id: usize,
        peer: PeerIdentity,
        address: PeerAddress,
        deadline: Instant,
    },
    Closed(usize),
}

pub struct PendingDialer {
    pub calls: AtomicUsize,
    events: mpsc::UnboundedSender<Event>,
}
impl PendingDialer {
    pub fn new() -> (Self, mpsc::UnboundedReceiver<Event>) {
        let (events, receiver) = mpsc::unbounded_channel();
        (
            Self {
                calls: AtomicUsize::default(),
                events,
            },
            receiver,
        )
    }
}
impl PeerDialer for PendingDialer {
    fn dial<'a>(
        &'a self,
        peer: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async move {
            let id = self.calls.fetch_add(DIAL_STEP, Ordering::SeqCst) + DIAL_STEP;
            let (stream, _remote) = tokio::io::duplex(DIALER_BUFFER_BYTES);
            let events = self.events.clone();
            let _owned = PeerIo::new(stream, move || {
                let _ = events.send(Event::Closed(id));
            });
            let _ = self.events.send(Event::Started {
                id,
                peer,
                address: address.clone(),
                deadline,
            });
            std::future::pending().await
        })
    }
}
