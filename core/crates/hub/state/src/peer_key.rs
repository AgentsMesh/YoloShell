use crate::{HubState, constants::*, record::Record};
use anyhow::{Result, ensure};
use yolo_hub_delegation::{DelegationError, ED25519_SIGNATURE_BYTES, PeerIdentity, Signer};

impl HubState {
    pub fn peer_identity(&self) -> PeerIdentity {
        PeerIdentity::new(self.hub, self.public_key().try_into().expect(KEY_FAILURE))
            .expect(KEY_FAILURE)
    }

    pub fn transport_key(&self) -> Result<russh::keys::PrivateKey> {
        self.verify()?;
        let record: Record = self
            .namespace
            .read_json(&self.path.join(HUB_IDENTITY_FILE))?;
        ensure!(record.hub_id == self.hub, KEY_FAILURE);
        let key = russh::keys::pkcs8::decode_pkcs8(&record.key, None)?;
        ensure!(
            key.public_key()
                .key_data()
                .ed25519()
                .is_some_and(|key| key.0.as_slice() == self.public_key()),
            KEY_FAILURE
        );
        self.verify()?;
        Ok(key)
    }
}

impl Signer for HubState {
    fn identity(&self) -> PeerIdentity {
        self.peer_identity()
    }
    fn sign(&self, message: &[u8]) -> Result<[u8; ED25519_SIGNATURE_BYTES], DelegationError> {
        HubState::sign(self, message)
            .map_err(|_| DelegationError::Signing)?
            .as_ref()
            .try_into()
            .map_err(|_| DelegationError::Signing)
    }
}
