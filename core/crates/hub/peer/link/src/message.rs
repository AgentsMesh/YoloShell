use crate::LinkError;
use serde::{Deserialize, Serialize};
use yolo_hub_delegation::{PeerIdentity, Signer};
use yolo_hub_identity::HubId;

pub(crate) const NONCE_BYTES: usize = 32;
pub(crate) const ROLE_CLIENT: &str = "client";
pub(crate) const ROLE_SERVER: &str = "server";
const HELLO_VERSION: u64 = 1;
const SIGNATURE_BYTES: usize = 64;
const DOMAIN: &str = "yolo.hub.peer.hello.v1";

#[derive(Serialize, Deserialize)]
struct WireIdentity {
    hub: HubId,
    #[serde(with = "serde_bytes")]
    public_key: Vec<u8>,
}

impl WireIdentity {
    fn new(identity: PeerIdentity) -> Self {
        Self {
            hub: identity.hub(),
            public_key: identity.key().to_vec(),
        }
    }

    fn identity(&self) -> Result<PeerIdentity, LinkError> {
        let key: [u8; 32] = self
            .public_key
            .as_slice()
            .try_into()
            .map_err(|_| LinkError::Identity)?;
        PeerIdentity::new(self.hub, key).map_err(|_| LinkError::Identity)
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Hello {
    version: u64,
    role: String,
    identity: WireIdentity,
    #[serde(with = "serde_bytes")]
    nonce: Vec<u8>,
    #[serde(with = "serde_bytes")]
    peer_nonce: Vec<u8>,
    #[serde(with = "serde_bytes")]
    signature: Vec<u8>,
}

#[derive(Serialize)]
struct Signable<'a> {
    domain: &'static str,
    version: u64,
    role: &'a str,
    identity: &'a WireIdentity,
    #[serde(with = "serde_bytes")]
    nonce: &'a [u8],
    #[serde(with = "serde_bytes")]
    peer_nonce: &'a [u8],
}

impl Hello {
    pub(crate) fn new(
        role: &str,
        identity: PeerIdentity,
        nonce: [u8; NONCE_BYTES],
        peer_nonce: Option<[u8; NONCE_BYTES]>,
        signer: &impl Signer,
    ) -> Result<Self, LinkError> {
        if signer.identity() != identity {
            return Err(LinkError::Identity);
        }
        let wire = WireIdentity::new(identity);
        let peer_nonce = peer_nonce.map_or_else(Vec::new, |nonce| nonce.to_vec());
        let signature = signer
            .sign(&signing_bytes(role, &wire, &nonce, &peer_nonce)?)
            .map_err(|_| LinkError::Signature)?
            .to_vec();
        Ok(Self {
            version: HELLO_VERSION,
            role: role.into(),
            identity: wire,
            nonce: nonce.to_vec(),
            peer_nonce,
            signature,
        })
    }

    /// Verifies the signature with the identity presented in the hello and
    /// returns it. Callers then decide whether that identity is acceptable.
    pub(crate) fn verify_signature(
        &self,
        role: &str,
        peer_nonce: Option<[u8; NONCE_BYTES]>,
    ) -> Result<PeerIdentity, LinkError> {
        if self.version != HELLO_VERSION {
            return Err(LinkError::Version);
        }
        if self.role != role {
            return Err(LinkError::Role);
        }
        let identity = self.identity.identity()?;
        let nonce = self.nonce()?;
        let expected = peer_nonce.map_or_else(Vec::new, |nonce| nonce.to_vec());
        if self.peer_nonce != expected {
            return Err(LinkError::Identity);
        }
        if self.signature.len() != SIGNATURE_BYTES {
            return Err(LinkError::Signature);
        }
        identity
            .verify(
                &signing_bytes(&self.role, &self.identity, &nonce, &self.peer_nonce)?,
                &self.signature,
            )
            .map_err(|_| LinkError::Signature)?;
        Ok(identity)
    }

    pub(crate) fn nonce(&self) -> Result<[u8; NONCE_BYTES], LinkError> {
        self.nonce
            .as_slice()
            .try_into()
            .map_err(|_| LinkError::Frame)
    }


}

fn signing_bytes(
    role: &str,
    identity: &WireIdentity,
    nonce: &[u8],
    peer_nonce: &[u8],
) -> Result<Vec<u8>, LinkError> {
    yolo_capnp_codec::canonical(&Signable {
        domain: DOMAIN,
        version: HELLO_VERSION,
        role,
        identity,
        nonce,
        peer_nonce,
    })
    .map_err(|_| LinkError::Frame)
}
