use super::{
    constants::STEP,
    fixture::{Fixture, empty_catalogue},
};
use crate::{ObservationStore, Retirement, checkpoint::Checkpoint, test_hooks::install};
use std::fs;
use yolo_hub_federation_settings::routing_limits;
use yolo_hub_peer_protocol_test_support::support::Identity;
use yolo_paths::directory::PrivateDirectory;

fn open(fixture: &Fixture, retirement: &Retirement) -> anyhow::Result<ObservationStore> {
    ObservationStore::open_with_retirement(
        PrivateDirectory::open(&fixture.path)?,
        fixture.peer.entry.identity,
        routing_limits(),
        retirement.clone(),
    )
}

#[test]
fn retired_generation_cannot_initialize_storage_or_become_active_again() {
    let fixture = Fixture::new();
    let retirement = Retirement::default();
    assert!(!retirement.is_retired());
    retirement.clone().retire();
    assert!(retirement.is_retired());
    assert!(open(&fixture, &retirement).is_err());
    retirement.retire();
    assert!(open(&fixture, &retirement).is_err());
    assert_eq!(fs::read_dir(&fixture.path).unwrap().count(), 0);
    assert!(
        fixture
            .open()
            .unwrap()
            .history()
            .unwrap()
            .catalogues()
            .next()
            .is_none()
    );
}

#[test]
fn retirement_revokes_reads_and_writes_without_releasing_an_owned_coordination_lock() {
    let fixture = Fixture::new();
    let retirement = Retirement::default();
    let mut store = open(&fixture, &retirement).unwrap();
    store.record_history(&fixture.history()).unwrap();
    let path = fixture.record_path(fixture.peer.origin.identity.hub());
    let version = fixture.version(&path);
    retirement.retire();
    assert!(store.history().is_err());
    assert!(
        store
            .record_history(&fixture.with(vec![fixture.revised(STEP + STEP, STEP)]))
            .is_err()
    );
    assert_eq!(fixture.version(&path), version);
    assert!(fixture.open().is_err());
    drop(store);
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}

#[test]
fn retirement_at_record_commit_fence_preserves_the_last_durable_snapshot() {
    let fixture = Fixture::new();
    let retirement = Retirement::default();
    let mut store = open(&fixture, &retirement).unwrap();
    store.record_history(&fixture.history()).unwrap();
    let path = fixture.record_path(fixture.peer.origin.identity.hub());
    let version = fixture.version(&path);
    let fence = retirement.clone();
    let guard = install(move |point| {
        if point == Checkpoint::BeforeRecordCommit {
            fence.retire();
        }
        Ok(())
    });
    assert!(
        store
            .record_history(&fixture.with(vec![fixture.revised(STEP + STEP, STEP)]))
            .is_err()
    );
    drop(guard);
    assert_eq!(fixture.version(&path), version);
    drop(store);
    assert_eq!(
        fixture.open().unwrap().history().unwrap(),
        fixture.history()
    );
}

#[test]
fn retirement_after_new_record_commit_cannot_publish_membership_or_replay_it() {
    let fixture = Fixture::new();
    let retirement = Retirement::default();
    let mut store = open(&fixture, &retirement).unwrap();
    let origin = Identity::new();
    let candidate = fixture.with(vec![empty_catalogue(&origin)]);
    let index = fixture.version(&fixture.index_path());
    let fence = retirement.clone();
    let guard = install(move |point| {
        if point == Checkpoint::AfterRecordCommit {
            fence.retire();
        }
        Ok(())
    });
    assert!(store.record_history(&candidate).is_err());
    drop(guard);
    assert_eq!(fixture.version(&fixture.index_path()), index);
    let path = fixture.record_path(origin.identity.hub());
    let record = fixture.version(&path);
    drop(store);
    let mut successor = fixture.open().unwrap();
    assert!(successor.history().unwrap().catalogues().next().is_none());
    assert!(successor.record_history(&candidate).is_err());
    assert_eq!(fixture.version(&path), record);
    assert_eq!(fixture.version(&fixture.index_path()), index);
}
