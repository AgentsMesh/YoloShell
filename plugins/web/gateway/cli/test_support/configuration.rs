use super::constants::*;
use ring::signature::{Ed25519KeyPair, KeyPair};
use russh::keys::{PrivateKey, pkcs8::decode_pkcs8, ssh_key::LineEnding};
use std::{
    ffi::OsString,
    fs,
    io::Write,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::{Path, PathBuf},
    sync::Arc,
};
use yolo_hub_client_access_protocol::ClientIdentity;
use yolo_hub_client_transport::HubPin;
use yolo_hub_identity::HubId;

pub struct Configuration {
    pub root: tempfile::TempDir,
    pub profile: PathBuf,
    pub key: PathBuf,
    pub token: PathBuf,
    pub socket: PathBuf,
    pub identity: ClientIdentity,
    pub hub: HubPin,
    pub hub_key: Arc<PrivateKey>,
}

impl Configuration {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        fs::set_permissions(root.path(), fs::Permissions::from_mode(DIRECTORY_MODE)).unwrap();
        let profile = root.path().join("profile.json");
        let key = root.path().join("client.key");
        let token = root.path().join("token");
        let socket = root.path().join("public-client.sock");
        let (client_public, client_key) = keys();
        let (public_key, hub_key) = keys();
        let hub = HubPin {
            hub_id: HubId::new(),
            public_key,
        };
        let identity = ClientIdentity {
            client_id: uuid::Uuid::new_v4(),
            public_key: client_public,
        };
        let profile_value = serde_json::json!({ SOCKET_FIELD: socket, HUB_FIELD: hub.hub_id,
            HUB_KEY_FIELD: hub.public_key, CLIENT_FIELD: identity });
        write_private(&profile, &serde_json::to_vec(&profile_value).unwrap());
        write_private(
            &key,
            client_key.to_openssh(LineEnding::LF).unwrap().as_bytes(),
        );
        write_private(&token, TOKEN.as_bytes());
        Self {
            root,
            profile,
            key,
            token,
            socket,
            identity,
            hub,
            hub_key,
        }
    }

    pub fn arguments(&self) -> Vec<OsString> {
        [
            OsString::from("yolo"),
            "web".into(),
            PROFILE_OPTION.into(),
            self.profile.as_os_str().into(),
            KEY_OPTION.into(),
            self.key.as_os_str().into(),
            TOKEN_OPTION.into(),
            self.token.as_os_str().into(),
            ORIGIN_OPTION.into(),
            ORIGIN.into(),
        ]
        .into()
    }

    pub fn corrupt_profile(&self, field: &str, value: serde_json::Value) {
        let mut profile: serde_json::Value =
            serde_json::from_slice(&fs::read(&self.profile).unwrap()).unwrap();
        profile[field] = value;
        write_private(&self.profile, &serde_json::to_vec(&profile).unwrap());
    }
}

pub fn write_private(path: &Path, bytes: &[u8]) {
    fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .mode(PRIVATE_MODE)
        .open(path)
        .unwrap()
        .write_all(bytes)
        .unwrap();
}

fn keys() -> ([u8; 32], Arc<PrivateKey>) {
    let document = Ed25519KeyPair::generate_pkcs8(&ring::rand::SystemRandom::new()).unwrap();
    let pair = Ed25519KeyPair::from_pkcs8(document.as_ref()).unwrap();
    (
        pair.public_key().as_ref().try_into().unwrap(),
        Arc::new(decode_pkcs8(document.as_ref(), None).unwrap()),
    )
}
