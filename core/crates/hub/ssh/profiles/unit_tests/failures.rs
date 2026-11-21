use super::{
    constants::*,
    fixture::{Fixture, write_new},
};
use crate::{
    RevisionExhausted,
    checkpoint::{Checkpoint, tests::install},
};
use std::fs;
use yolo_hub_ssh_profile_protocol_test_support::{change, configuration};

#[test]
fn failed_commit_and_lost_reply_poison_the_owner_but_reopen_recovers_exact_durable_receipt() {
    for point in [Checkpoint::BeforeCommit, Checkpoint::AfterCommit] {
        let fixture = Fixture::new();
        let mut store = fixture.open(None).unwrap();
        let before = store.current().unwrap();
        let request = change(&before, configuration());
        let guard = install(move |actual| {
            anyhow::ensure!(actual != point, LOST_ACK);
            Ok(())
        });
        assert!(store.apply(request.clone()).is_err());
        drop(guard);
        assert!(store.current().is_err());
        assert!(store.verify().is_err());
        assert!(store.apply(request.clone()).is_err());
        drop(store);
        let mut reopened = fixture.open(None).unwrap();
        let observed = reopened.current().unwrap();
        if point == Checkpoint::BeforeCommit {
            assert_eq!(observed, before);
        } else {
            assert_eq!(observed.last_change, request.request_id);
            assert_eq!(observed.configuration, request.configuration);
            let bytes = fixture.bytes();
            assert_eq!(reopened.apply(request).unwrap(), observed);
            assert_eq!(fixture.bytes(), bytes);
        }
    }
}

#[test]
fn identical_record_substitution_inside_commit_fence_never_overwrites_replacement() {
    let fixture = Fixture::new();
    let mut store = fixture.open(None).unwrap();
    let previous = store.current().unwrap();
    let request = change(&previous, configuration());
    let path = fixture.record();
    let displaced = fixture.sibling(DISPLACED);
    let bytes = fixture.bytes();
    let guard = install(move |point| {
        if point == Checkpoint::BeforeCommit {
            fs::rename(&path, &displaced)?;
            write_new(&path, &bytes);
        }
        Ok(())
    });
    assert!(store.apply(request).is_err());
    drop(guard);
    assert!(store.current().is_err());
    drop(store);
    assert_eq!(fixture.open(None).unwrap().current().unwrap(), previous);
}

#[test]
fn exhausted_revision_is_a_nonmutating_error_and_does_not_poison_valid_state() {
    let fixture = Fixture::new();
    let store = fixture.open(None).unwrap();
    let mut record = store.record.clone();
    record.state.revision = u64::MAX;
    drop(store);
    fs::write(
        fixture.record(),
        serde_json::to_vec_pretty(&record).unwrap(),
    )
    .unwrap();
    let mut store = fixture.open(None).unwrap();
    let current = store.current().unwrap();
    let bytes = fixture.bytes();
    assert!(
        store
            .apply(change(&current, configuration()))
            .unwrap_err()
            .is::<RevisionExhausted>()
    );
    assert_eq!(store.current().unwrap(), current);
    assert_eq!(fixture.bytes(), bytes);
}
