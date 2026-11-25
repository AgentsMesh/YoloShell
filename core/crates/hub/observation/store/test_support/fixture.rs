use super::constants::*;
use anyhow::Result;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_directory::ObservationHistory;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_identity::HubId;
use yolo_hub_observation_store::{OBSERVATION_DIRECTORY, OBSERVATION_INDEX_FILE, ObservationStore};
use yolo_hub_peer_protocol::{CATALOGUE_SCHEMA_VERSION, Catalogue, Purpose, Signed};
use yolo_hub_peer_protocol_test_support::{self as peer_fixture, support::Identity};
use yolo_paths::{
    directory::PrivateDirectory, identity::FileIdentity, namespace_lease::NamespaceLease,
};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub path: PathBuf,
    pub peer: peer_fixture::Fixture,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let path = root
            .path()
            .canonicalize()
            .unwrap()
            .join(OBSERVATION_DIRECTORY);
        PrivateDirectory::open_or_create_strict(&path).unwrap();
        Self {
            root,
            path,
            peer: peer_fixture::directory::fixture(Some(peer_fixture::directory::AGENT_DIRECTORY)),
        }
    }
    pub fn open(&self) -> Result<ObservationStore> {
        ObservationStore::open(
            PrivateDirectory::open(&self.path)?,
            self.peer.entry.identity,
            routing_limits(),
        )
    }
    pub fn history(&self) -> ObservationHistory {
        self.with(vec![self.peer.export.catalogue.clone()])
    }
    pub fn with(&self, catalogues: Vec<Signed<Catalogue>>) -> ObservationHistory {
        ObservationHistory::new(self.peer.entry.identity, catalogues, routing_limits()).unwrap()
    }
    pub fn index_path(&self) -> PathBuf {
        self.path.join(OBSERVATION_INDEX_FILE)
    }
    pub fn record_path(&self, origin: HubId) -> PathBuf {
        self.path
            .join(origin.to_string())
            .with_extension(ORIGIN_EXTENSION)
    }
    pub fn namespace(&self) -> NamespaceLease {
        NamespaceLease::capture(&[(OBSERVATION_DIRECTORY, &self.path)]).unwrap()
    }
    pub fn version(&self, path: &Path) -> FileIdentity {
        self.namespace()
            .read_json_with_identity::<serde_json::Value>(path)
            .unwrap()
            .1
    }
    pub fn sibling(&self, name: &str) -> PathBuf {
        self.path.parent().unwrap().join(name)
    }
    pub fn revised(&self, revision: u64, policy_epoch: u64) -> Signed<Catalogue> {
        let mut payload = self.peer.export.catalogue.payload.clone();
        payload.revision = revision;
        payload.policy_epoch = policy_epoch;
        Signed::new(payload, Purpose::Catalogue, &self.peer.origin).unwrap()
    }
}
pub fn write_new(path: &Path, bytes: &[u8]) {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(FILE_MODE)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}
pub fn replace_same(path: &Path, displaced: &Path) {
    let bytes = fs::read(path).unwrap();
    fs::rename(path, displaced).unwrap();
    write_new(path, &bytes);
}
pub fn empty_catalogue(identity: &Identity) -> Signed<Catalogue> {
    Signed::new(
        Catalogue {
            schema_version: CATALOGUE_SCHEMA_VERSION,
            revision: STEP,
            policy_epoch: STEP,
            issued_at_ms: STEP,
            expires_at_ms: STEP + STEP,
            terminals: vec![],
            agents: vec![],
            runs: vec![],
            workspaces: vec![],
        },
        Purpose::Catalogue,
        identity,
    )
    .unwrap()
}
pub fn large_catalogue(identity: &Identity, count: usize) -> Signed<Catalogue> {
    let mut payload = empty_catalogue(identity).payload;
    payload.terminals = (0..count)
        .map(|_| {
            let mut fields = peer_fixture::session(identity.identity).fields().clone();
            fields.cwd = format!("/{}", DIRECTORY_FILL.repeat(DIRECTORY_BYTES - 1));
            yolo_hub_terminal_protocol::TerminalSummary::new(fields).unwrap()
        })
        .collect();
    payload
        .terminals
        .sort_by_key(|session| session.fields().address);
    Signed::new(payload, Purpose::Catalogue, identity).unwrap()
}
pub fn rotated(original: PeerIdentity) -> Identity {
    let mut identity = Identity::new();
    identity.identity = PeerIdentity::new(original.hub(), *identity.identity.key()).unwrap();
    identity
}
