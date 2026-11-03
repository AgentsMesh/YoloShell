use super::*;
use yolo_hub_delegation::ED25519_SIGNATURE_BYTES;
use yolo_hub_state::HubState;
mod durable_binding;

const STATE_DIR: &str = "hub-state";
const HOST_DIR: &str = "host-home";
const PUBLICATION_DIR: &str = "publication";

#[test]
fn policy_epoch_survives_reopen_and_advances_when_configuration_changes() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let state = HubState::open(&root.join(STATE_DIR), &root.join(HOST_DIR)).unwrap();
    let directory = state.directory().unwrap();
    let path = directory.path().join(PUBLICATION_DIR);
    let config = Configuration::default();
    let publication = Publication::open(
        directory.ensure_descendant(&path).unwrap(),
        state.peer_identity(),
        &config,
    )
    .unwrap();
    assert_eq!(publication.policy_epoch(), FIRST_POLICY_EPOCH);
    publication.verify().unwrap();
    assert!(publication.latest().is_none());
    assert!(
        Publication::open(
            directory.open_descendant(&path).unwrap(),
            state.peer_identity(),
            &config
        )
        .is_err()
    );
    drop(publication);
    let publication = Publication::open(
        directory.open_descendant(&path).unwrap(),
        state.peer_identity(),
        &config,
    )
    .unwrap();
    assert_eq!(publication.policy_epoch(), FIRST_POLICY_EPOCH);
    drop(publication);
    let mut changed = config;
    changed.listen = Some(std::net::SocketAddr::new(
        std::net::Ipv4Addr::LOCALHOST.into(),
        u16::default(),
    ));
    let publication = Publication::open(
        directory.open_descendant(&path).unwrap(),
        state.peer_identity(),
        &changed,
    )
    .unwrap();
    assert_eq!(
        publication.policy_epoch(),
        FIRST_POLICY_EPOCH + FIRST_POLICY_EPOCH
    );
}

#[test]
fn a_legacy_catalogue_shape_is_dropped_and_keeps_the_revision() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let state = HubState::open(&root.join(STATE_DIR), &root.join(HOST_DIR)).unwrap();
    let directory = state.directory().unwrap();
    let path = directory.path().join(PUBLICATION_DIR);
    let config = Configuration::default();
    let publication = Publication::open(
        directory.ensure_descendant(&path).unwrap(),
        state.peer_identity(),
        &config,
    )
    .unwrap();
    let mut record = publication.record.clone();
    record.latest = Some(Signed {
        signer: state.peer_identity(),
        purpose: Purpose::Catalogue,
        payload: Catalogue {
            schema_version: 0,
            revision: record.revision,
            policy_epoch: record.policy_epoch,
            issued_at_ms: 1,
            expires_at_ms: 2,
            terminals: Vec::new(),
            agents: Vec::new(),
            runs: Vec::new(),
            workspaces: Vec::new(),
        },
        signature: vec![0; ED25519_SIGNATURE_BYTES],
    });
    publication
        .namespace
        .write_json(&publication.path, &record)
        .unwrap();
    let revision = record.revision;
    drop(publication);
    let publication = Publication::open(
        directory.open_descendant(&path).unwrap(),
        state.peer_identity(),
        &config,
    )
    .unwrap();
    assert!(publication.latest().is_none());
    assert_eq!(publication.record.revision, revision);
}

#[test]
fn corrupted_publication_identity_is_not_silently_reinitialized() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let state = HubState::open(&root.join(STATE_DIR), &root.join(HOST_DIR)).unwrap();
    let directory = state.directory().unwrap();
    let path = directory.path().join(PUBLICATION_DIR);
    let config = Configuration::default();
    let publication = Publication::open(
        directory.ensure_descendant(&path).unwrap(),
        state.peer_identity(),
        &config,
    )
    .unwrap();
    let mut record = publication.record.clone();
    record.policy_epoch = 0;
    publication
        .namespace
        .write_json(&publication.path, &record)
        .unwrap();
    drop(publication);
    assert!(
        Publication::open(
            directory.open_descendant(&path).unwrap(),
            state.peer_identity(),
            &config
        )
        .is_err()
    );
}
