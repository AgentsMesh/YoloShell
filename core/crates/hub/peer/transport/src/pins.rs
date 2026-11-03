use crate::constants::INVALID_PINS;
use anyhow::{Result, ensure};
use russh::keys::PublicKey;
use yolo_hub_delegation::PeerIdentity;

#[derive(Clone)]
pub struct PeerPins(Vec<PeerIdentity>);

impl PeerPins {
    pub fn new(peers: Vec<PeerIdentity>) -> Result<Self> {
        for (index, peer) in peers.iter().enumerate() {
            ensure!(
                !peers[..index]
                    .iter()
                    .any(|other| other.hub() == peer.hub() || other.key() == peer.key()),
                INVALID_PINS
            );
        }
        Ok(Self(peers))
    }
    pub(crate) fn identify(&self, key: &PublicKey) -> Option<PeerIdentity> {
        self.0.iter().find(|peer| matches_key(**peer, key)).copied()
    }
}
pub(crate) fn matches_key(identity: PeerIdentity, key: &PublicKey) -> bool {
    key.key_data()
        .ed25519()
        .is_some_and(|key| &key.0 == identity.key())
}
