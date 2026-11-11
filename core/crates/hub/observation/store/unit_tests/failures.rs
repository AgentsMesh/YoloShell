use super::{
    constants::*,
    fixture::{Fixture, empty_catalogue, replace_same},
};
use crate::{checkpoint::Checkpoint, test_hooks::install};
use std::fs;
use yolo_hub_peer_protocol_test_support::support::Identity;

#[test]
fn interrupted_new_origin_commit_keeps_confirmed_origins_readable_and_never_adopts_orphan() {
    for point in [
        Checkpoint::BeforeRecordCommit,
        Checkpoint::AfterRecordCommit,
        Checkpoint::AfterIndexCommit,
    ] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        let committed = fixture.history();
        store.record_history(&committed).unwrap();
        let new = Identity::new();
        let candidate = empty_catalogue(&new);
        let next = fixture.with(vec![candidate.clone()]);
        let index_before = fs::read(fixture.index_path()).unwrap();
        let guard = install(move |actual| {
            anyhow::ensure!(actual != point, LOST_ACK);
            Ok(())
        });
        assert!(store.record_history(&next).is_err());
        drop(guard);
        assert!(store.history().is_err());
        drop(store);
        let mut reopened = fixture.open().unwrap();
        if point == Checkpoint::AfterIndexCommit {
            assert_eq!(
                reopened.history().unwrap(),
                fixture.with(vec![fixture.peer.export.catalogue.clone(), candidate])
            );
        } else {
            assert_eq!(reopened.history().unwrap(), committed);
            assert_eq!(fs::read(fixture.index_path()).unwrap(), index_before);
        }
        let path = fixture.record_path(new.identity.hub());
        if point == Checkpoint::AfterRecordCommit {
            let bytes = fs::read(&path).unwrap();
            let version = fixture.version(&path);
            assert!(reopened.record_history(&next).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
            assert_eq!(fixture.version(&path), version);
            drop(reopened);
            assert_eq!(fixture.open().unwrap().history().unwrap(), committed);
        }
    }
}

#[test]
fn lost_ack_after_existing_origin_write_recovers_the_new_signed_snapshot() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let next = fixture.with(vec![fixture.revised(STEP + STEP, STEP)]);
    let index = fixture.version(&fixture.index_path());
    let guard = install(|point| {
        anyhow::ensure!(point != Checkpoint::AfterRecordCommit, LOST_ACK);
        Ok(())
    });
    assert!(store.record_history(&next).is_err());
    drop(guard);
    assert!(store.history().is_err());
    drop(store);
    assert_eq!(fixture.open().unwrap().history().unwrap(), next);
    assert_eq!(fixture.version(&fixture.index_path()), index);
}

#[test]
fn same_byte_replacement_inside_commit_fence_is_not_overwritten() {
    for change_index in [false, true] {
        let fixture = Fixture::new();
        let mut store = fixture.open().unwrap();
        store.record_history(&fixture.history()).unwrap();
        let next = fixture.with(vec![fixture.revised(STEP + STEP, STEP)]);
        let path = if change_index {
            fixture.index_path()
        } else {
            fixture.record_path(fixture.peer.origin.identity.hub())
        };
        let bytes = fs::read(&path).unwrap();
        let displaced = fixture.sibling(DISPLACED);
        let target = path.clone();
        let guard = install(move |point| {
            if point == Checkpoint::BeforeRecordCommit {
                replace_same(&target, &displaced);
            }
            Ok(())
        });
        assert!(store.record_history(&next).is_err());
        drop(guard);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert!(store.history().is_err());
        assert!(store.record_history(&next).is_err());
    }
}

#[test]
fn a_new_shard_replaced_before_index_commit_is_never_adopted() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let committed = fixture.history();
    store.record_history(&committed).unwrap();
    let index = fixture.version(&fixture.index_path());
    let other = Identity::new();
    let path = fixture.record_path(other.identity.hub());
    let displaced = fixture.sibling(DISPLACED);
    let target = path.clone();
    let guard = install(move |point| {
        if point == Checkpoint::AfterRecordCommit {
            replace_same(&target, &displaced);
        }
        Ok(())
    });
    assert!(
        store
            .record_history(&fixture.with(vec![empty_catalogue(&other)]))
            .is_err()
    );
    drop(guard);
    let replacement = (fs::read(&path).unwrap(), fixture.version(&path));
    assert!(store.history().is_err());
    drop(store);
    assert_eq!(fixture.open().unwrap().history().unwrap(), committed);
    assert_eq!(fixture.version(&fixture.index_path()), index);
    assert_eq!(
        (fs::read(&path).unwrap(), fixture.version(&path)),
        replacement
    );
}
