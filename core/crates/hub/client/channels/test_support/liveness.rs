use crate::{
    harness::Harness,
    operations,
    peer::{Peer, hello_reply},
};
use yolo_hub_client_channels::CONTROL_KEEPALIVE_INTERVAL;

pub async fn ready(harness: &mut Harness, peer: &mut Peer, generation: uuid::Uuid) {
    let reply = hello_reply(harness.endpoint.hub);
    operations::round_trip(
        harness,
        peer,
        generation,
        yolo_hub_rpc_protocol::Command::Hello {},
        reply,
    )
    .await;
}

pub async fn idle() {
    // Advance only while no socket exchange is awaiting a peer response.
    // Resume before real Unix I/O so Tokio cannot auto-jump past its RPC deadline.
    tokio::time::pause();
    tokio::time::advance(CONTROL_KEEPALIVE_INTERVAL).await;
    tokio::time::resume();
}
