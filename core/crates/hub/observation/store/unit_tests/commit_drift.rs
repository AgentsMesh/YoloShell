use super::{
    constants::*,
    fixture::{Fixture, empty_catalogue, replace_same},
};
use crate::{checkpoint::Checkpoint, test_hooks::install};
use std::fs;
use yolo_hub_peer_protocol_test_support::support::Identity;

#[test]
fn index_or_retained_shard_replacement_after_commit_cannot_publish_new_membership() {
    for replace_index in [false, true] {
        rejects_replacement_before_index_commit(replace_index);
    }
}

fn rejects_replacement_before_index_commit(replace_index: bool) {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    let committed = fixture.history();
    store.record_history(&committed).unwrap();
    let origin = Identity::new();
    let candidate = fixture.with(vec![empty_catalogue(&origin)]);
    let target = if replace_index {
        fixture.index_path()
    } else {
        fixture.record_path(fixture.peer.origin.identity.hub())
    };
    let replacement_path = target.clone();
    let original_index = fs::read(fixture.index_path()).unwrap();
    let displaced = fixture.sibling(DISPLACED);
    let guard = install(move |point| {
        if point == Checkpoint::AfterRecordCommit {
            replace_same(&target, &displaced);
        }
        Ok(())
    });
    assert!(store.record_history(&candidate).is_err());
    drop(guard);
    let orphan_path = fixture.record_path(origin.identity.hub());
    let orphan = (
        fs::read(&orphan_path).unwrap(),
        fixture.version(&orphan_path),
    );
    let replacement = fixture.version(&replacement_path);
    let index = fixture.version(&fixture.index_path());
    assert_eq!(fs::read(fixture.index_path()).unwrap(), original_index);
    assert!(store.history().is_err());
    assert!(store.record_history(&candidate).is_err());
    drop(store);
    let mut successor = fixture.open().unwrap();
    assert_eq!(successor.history().unwrap(), committed);
    assert!(successor.record_history(&candidate).is_err());
    assert_eq!(fixture.version(&replacement_path), replacement);
    assert_eq!(fixture.version(&fixture.index_path()), index);
    assert_eq!(
        (
            fs::read(&orphan_path).unwrap(),
            fixture.version(&orphan_path)
        ),
        orphan
    );
}

#[test]
fn existing_shard_corruption_after_commit_cannot_be_acknowledged_as_durable_history() {
    let fixture = Fixture::new();
    let mut store = fixture.open().unwrap();
    store.record_history(&fixture.history()).unwrap();
    let path = fixture.record_path(fixture.peer.origin.identity.hub());
    let index = fixture.version(&fixture.index_path());
    let target = path.clone();
    let guard = install(move |point| {
        if point == Checkpoint::AfterRecordCommit {
            fs::write(&target, INVALID_BYTES)?;
        }
        Ok(())
    });
    let newer = fixture.with(vec![fixture.revised(STEP + STEP, STEP)]);
    assert!(store.record_history(&newer).is_err());
    drop(guard);
    assert!(store.history().is_err());
    assert!(store.record_history(&newer).is_err());
    drop(store);
    assert!(fixture.open().is_err());
    assert_eq!(fs::read(&path).unwrap(), INVALID_BYTES);
    assert_eq!(fixture.version(&fixture.index_path()), index);
}
