use russh::keys::PublicKey;
use yolo_hub_client_access_protocol::ED25519_PUBLIC_KEY_BYTES;
use yolo_hub_identity::HubId;

/// Enrollment binds this Hub identity to this key before connecting.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HubPin {
    pub hub_id: HubId,
    pub public_key: [u8; ED25519_PUBLIC_KEY_BYTES],
}

impl HubPin {
    pub(crate) fn matches(&self, key: &PublicKey) -> bool {
        key.key_data()
            .ed25519()
            .is_some_and(|key| key.0 == self.public_key)
    }
}
