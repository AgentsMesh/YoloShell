use super::{constants::*, fixture::Fixture};
use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8, ssh_key::LineEnding};
use std::{os::unix::fs::OpenOptionsExt, path::Path, sync::Arc};
use yolo_hub_client_access_protocol::{ClientGrant, ClientIdentity, OriginAccess};
use yolo_hub_routing::Right;

pub fn private_file(path: &Path, content: &[u8]) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(PRIVATE_FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(content).unwrap();
    file.sync_all().unwrap();
}

pub fn client_key(path: &Path) -> (ClientIdentity, Arc<PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let signing = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    let key = decode_pkcs8(document.as_ref(), None).unwrap();
    let encoded = key.to_openssh(LineEnding::LF).unwrap();
    private_file(path, encoded.as_bytes());
    (
        ClientIdentity {
            client_id: uuid::Uuid::new_v4(),
            public_key: signing.public_key().as_ref().try_into().unwrap(),
        },
        Arc::new(key),
    )
}

impl Fixture {
    pub fn grant(&self, rights: &[Right]) -> ClientGrant {
        ClientGrant {
            identity: self.identity,
            origins: vec![OriginAccess {
                origin: self.hub,
                rights: rights.iter().copied().collect(),
            }],
            expires_at_ms: u64::try_from(yolo_runtime::model::now_ms()).unwrap()
                + u64::try_from(GRANT_LIFETIME.as_millis()).unwrap(),
        }
    }
}
