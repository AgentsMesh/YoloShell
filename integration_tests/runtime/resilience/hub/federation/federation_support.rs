#[path = "../connector/connector_federation_start.rs"]
mod prepared_start;
use std::{path::PathBuf, time::Duration};
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::{ExportPolicy, Peer, PeerAddress};
use yolo_hub_identity::HubId;
use yolo_hub_routing::Right;
use yolo_hub_state::HubState;

pub const GRAPH_TIMEOUT: Duration = Duration::from_secs(180);
pub const POLL: Duration = Duration::from_millis(100);
pub const TEMP_ROOT: &str = "/tmp";
pub const NAME_A: &str = "node-a-agent";
pub const NAME_B: &str = "node-b-agent";
pub const NAME_C: &str = "node-c-agent";
const REEXPORT_ALLOWED: u8 = 1;
const LEAF_EXPORT: u8 = 0;
const TEST_PROFILE: &str = "in-process";
const PROBE_TIMEOUT: Duration = Duration::from_secs(1);
const LIVE_SOCKET: &str = "another test Hub owns this peer socket";
pub fn deadline() -> Instant {
    Instant::now() + GRAPH_TIMEOUT
}
pub fn exports(origin: HubId, relay: bool) -> ExportPolicy {
    ExportPolicy {
        origin,
        rights: [
            Right::Observe,
            Right::TerminalRead,
            Right::TerminalWrite,
            Right::Lifecycle,
        ]
        .into_iter()
        .collect(),
        remaining_reexports: if relay { REEXPORT_ALLOWED } else { LEAF_EXPORT },
    }
}
pub fn peer(
    identity: PeerIdentity,
    dial: Option<&HubState>,
    consume: bool,
    exports: Vec<ExportPolicy>,
) -> Peer {
    peer_at(
        identity,
        dial.map(|state| {
            PeerAddress::ssh(TEST_PROFILE, state.peer_socket().unwrap()).unwrap()
        }),
        consume,
        exports,
    )
}

pub fn peer_at_socket(
    identity: PeerIdentity,
    socket: PathBuf,
    consume: bool,
    exports: Vec<ExportPolicy>,
) -> Peer {
    peer_at(
        identity,
        Some(PeerAddress::ssh(TEST_PROFILE, socket).unwrap()),
        consume,
        exports,
    )
}

pub fn peer_at(
    identity: PeerIdentity,
    dial: Option<PeerAddress>,
    consume: bool,
    exports: Vec<ExportPolicy>,
) -> Peer {
    Peer {
        identity,
        dial,
        consume,
        exports,
    }
}

#[path = "federation_support_running.rs"]
mod running;
pub use running::{DirectUnixDialer, RunningNode, publish};
