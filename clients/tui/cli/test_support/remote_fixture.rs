use crate::{arguments::Arguments, selection::Selection};
use clap::Parser;
use std::{
    ffi::OsString,
    fs::OpenOptions,
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use yolo_hub_client_transport::HubPin;
use yolo_hub_identity::HubId;
use yolo_hub_ssh_entry::{ENTRY_PROFILE_SCHEMA, EntryProfile};

#[path = "remote_arguments_constants.rs"]
mod arguments;
#[path = "remote_constants.rs"]
pub mod constants;
#[path = "remote_keys.rs"]
pub mod keys;
use constants::*;

#[derive(Clone, Copy)]
pub enum Input {
    Profile,
    AccountKey,
    ClientKey,
}

pub const INPUTS: [Input; 3] = [Input::Profile, Input::AccountKey, Input::ClientKey];
pub const KEYS: [Input; 2] = [Input::AccountKey, Input::ClientKey];

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub profile: EntryProfile,
    pub account: zeroize::Zeroizing<String>,
    pub client: zeroize::Zeroizing<String>,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        yolo_paths::directory::PrivateDirectory::ensure(&root.path().canonicalize().unwrap())
            .unwrap();
        let hub = keys::ed25519();
        let host = keys::ed25519();
        let profile = EntryProfile {
            schema_version: ENTRY_PROFILE_SCHEMA,
            host: HOST.into(),
            port: SSH_PORT,
            username: USERNAME.into(),
            outer_host_key: host.public_key().to_openssh().unwrap(),
            remote_socket: PUBLIC_SOCKET.into(),
            hub: HubPin {
                hub_id: HubId::new(),
                public_key: hub.public_key().key_data().ed25519().unwrap().0,
            },
            client_id: uuid::Uuid::new_v4(),
        };
        let fixture = Self {
            root,
            profile,
            account: keys::encode(&keys::ed25519()),
            client: keys::encode(&keys::ed25519()),
        };
        write_new(
            &fixture.path(Input::Profile),
            &serde_json::to_vec(&fixture.profile).unwrap(),
        );
        write_new(&fixture.path(Input::AccountKey), fixture.account.as_bytes());
        write_new(&fixture.path(Input::ClientKey), fixture.client.as_bytes());
        fixture
    }

    pub fn path(&self, input: Input) -> PathBuf {
        self.root.path().canonicalize().unwrap().join(match input {
            Input::Profile => PROFILE_FILE,
            Input::AccountKey => ACCOUNT_KEY_FILE,
            Input::ClientKey => CLIENT_KEY_FILE,
        })
    }

    pub fn select(&self) -> anyhow::Result<Selection> {
        self.select_paths(
            self.path(Input::Profile),
            self.path(Input::AccountKey),
            self.path(Input::ClientKey),
        )
    }

    pub fn select_paths(
        &self,
        profile: PathBuf,
        account: PathBuf,
        client: PathBuf,
    ) -> anyhow::Result<Selection> {
        let arguments: Vec<OsString> = vec![
            super::constants::PROGRAM.into(),
            arguments::REMOTE_PROFILE_ARGUMENT.into(),
            profile.into(),
            arguments::ACCOUNT_KEY_ARGUMENT.into(),
            account.into(),
            arguments::CLIENT_KEY_ARGUMENT.into(),
            client.into(),
        ];
        Arguments::try_parse_from(arguments)?.selection()
    }

    pub fn rejected(&self) -> anyhow::Error {
        let error = match self.select() {
            Err(error) => error,
            Ok(_) => panic!(),
        };
        self.private_error(&error);
        error
    }

    pub fn private_error(&self, error: &anyhow::Error) {
        let message = format!("{error:#}");
        for secret in [
            SECRET_MARKER,
            &self.account,
            &self.client,
            self.account.lines().nth(KEY_BODY_LINE).unwrap(),
            self.client.lines().nth(KEY_BODY_LINE).unwrap(),
        ] {
            assert!(!message.contains(secret));
        }
        for input in KEYS {
            let Ok(bytes) = std::fs::read(self.path(input)) else {
                continue;
            };
            let Ok(secret) = std::str::from_utf8(&bytes) else {
                continue;
            };
            if !secret.is_empty() {
                assert!(!message.contains(secret));
            }
            if let Some(body) = secret
                .lines()
                .nth(KEY_BODY_LINE)
                .filter(|body| !body.is_empty())
            {
                assert!(!message.contains(body));
            }
        }
    }
}

pub fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(PRIVATE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
    file.sync_all().unwrap();
}
