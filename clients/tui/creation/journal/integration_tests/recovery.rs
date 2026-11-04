use super::{filesystem::*, model::*};
use crate::{checkpoint::Checkpoint, test_hooks, *};
use std::os::unix::fs::OpenOptionsExt;
use yolo_hub_identity::HubId;
use yolo_paths::directory::PrivateDirectory;

const INJECTED_FAILURE: &str = "injected journal acknowledgement failure";
const DISPLACED_SUFFIX: &str = "displaced";

pub(super) fn fail_at(stage: Checkpoint) -> test_hooks::Guard {
    test_hooks::install(move |point| {
        if point == stage {
            anyhow::bail!(INJECTED_FAILURE);
        }
        Ok(())
    })
}

#[test]
fn failed_commit_fence_recovers_only_the_retained_previous_document() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let before = fixture.stored();
    let record = pending(HubId::new());
    let failure = fail_at(Checkpoint::BeforeCommit);
    assert!(journal.retain_confirmed(record.clone()).is_err());
    assert!(journal.pending().is_err());
    assert_eq!(fixture.stored(), before);
    drop(failure);
    journal.recover().unwrap();
    assert!(journal.pending().unwrap().is_empty());
    assert!(journal.retain_confirmed(record.clone()).unwrap());
    assert_eq!(journal.pending().unwrap(), [record]);
}

#[test]
fn exact_observed_commit_survives_lost_ack_without_republishing_or_losing_result() {
    for stage in [Checkpoint::CommitObserved, Checkpoint::WriteAcknowledged] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let record = pending(HubId::new());
        let failure = fail_at(stage);
        assert!(journal.retain_confirmed(record.clone()).is_err());
        assert!(journal.pending().is_err());
        let published = fixture.stored();
        drop(failure);
        journal.recover().unwrap();
        assert_eq!(journal.pending().unwrap(), [record.clone()]);
        assert!(!journal.retain_confirmed(record).unwrap());
        assert_eq!(fixture.stored(), published);
    }
}

#[test]
fn acknowledged_removal_with_lost_disk_ack_recovers_the_exact_empty_document() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let record = pending(HubId::new());
    journal.retain_confirmed(record.clone()).unwrap();
    let failure = fail_at(Checkpoint::WriteAcknowledged);
    assert!(journal.acknowledge(&record).is_err());
    drop(failure);
    journal.recover().unwrap();
    assert!(journal.pending().unwrap().is_empty());
    assert!(!journal.acknowledge(&record).unwrap());
}

#[test]
fn recovery_never_adopts_identical_replacement_of_old_or_observed_new_file() {
    for stage in [Checkpoint::BeforeCommit, Checkpoint::WriteAcknowledged] {
        let fixture = Fixture::new();
        let mut journal = fixture.open();
        let failure = fail_at(stage);
        assert!(journal.retain_confirmed(pending(HubId::new())).is_err());
        drop(failure);
        let replacement = fixture.stored();
        fixture.replace(&replacement);
        assert!(journal.recover().is_err());
        assert!(journal.pending().is_err());
        assert!(CreationJournal::open(fixture.parent(), fixture.entry).is_err());
        assert_eq!(fixture.stored(), replacement);
    }
}

#[test]
fn replaced_namespace_cannot_be_reopened_by_recovery() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let failure = fail_at(Checkpoint::WriteAcknowledged);
    assert!(journal.retain_confirmed(pending(HubId::new())).is_err());
    drop(failure);
    std::fs::rename(&fixture.path, fixture.root.path().join(MOVED_DIRECTORY)).unwrap();
    PrivateDirectory::ensure(&fixture.path).unwrap();
    assert!(journal.recover().is_err());
    assert!(journal.pending().is_err());
    assert!(!fixture.file().exists());
}

#[test]
fn interference_after_observer_does_not_turn_a_foreign_file_into_commit_evidence() {
    let fixture = Fixture::new();
    let mut journal = fixture.open();
    let path = fixture.file();
    let interference = test_hooks::install(move |point| {
        if point == Checkpoint::CommitObserved {
            let bytes = std::fs::read(&path)?;
            std::fs::rename(&path, path.with_extension(DISPLACED_SUFFIX))?;
            let mut replacement = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(PRIVATE_MODE)
                .open(&path)?;
            std::io::Write::write_all(&mut replacement, &bytes)?;
        }
        Ok(())
    });
    assert!(journal.retain_confirmed(pending(HubId::new())).is_err());
    drop(interference);
    let replacement = fixture.stored();
    assert!(journal.recover().is_err());
    assert_eq!(fixture.stored(), replacement);
}
