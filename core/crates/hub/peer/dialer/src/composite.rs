use crate::{PeerDialFuture, PeerDialer, SystemSshDialer};
use std::sync::Arc;
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::PeerAddress;

/// Dispatches user SSH links to the local `ssh` client and leaves managed
/// remote-setup profile links on the Hub broker transport.
pub struct CompositeDialer {
    ssh: SystemSshDialer,
    profile: Arc<dyn PeerDialer>,
}

impl CompositeDialer {
    pub fn new(ssh: SystemSshDialer, profile: Arc<dyn PeerDialer>) -> Self {
        Self { ssh, profile }
    }
}

impl PeerDialer for CompositeDialer {
    fn dial<'a>(
        &'a self,
        peer: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a> {
        match address {
            PeerAddress::Ssh(_) => self.ssh.dial(peer, address, deadline),
            PeerAddress::Profile(_) => self.profile.dial(peer, address, deadline),
        }
    }
}
