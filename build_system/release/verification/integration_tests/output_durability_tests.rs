use super::support::error_text;
use super::test_constants::{
    INJECTED_CANDIDATE_COMMIT_SYNC_FAILURE, INJECTED_PARENT_COMMIT_SYNC_FAILURE,
    OUTPUT_TEST_DESTINATION_NAME,
};
use crate::output_publish::{PublicationCheckpoint, PublicationSync, PublishHooks};
use crate::test_support::{OutputCleanup, assert_no_staging_name, publish_with_hooks};
use anyhow::{Result, bail};
use std::cell::RefCell;
use yolo_release_contracts::{COMMITTED_ACKNOWLEDGEMENT_ERROR, COMMITTED_VALIDATION_ERROR};
use yolo_release_filesystem::DirectoryHandle;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicationEvent {
    Checkpoint(PublicationCheckpoint),
    CandidateSync(PublicationSync),
    ParentSync(PublicationSync),
}

const EXPECTED_PUBLICATION_ORDER: &[PublicationEvent] = &[
    PublicationEvent::Checkpoint(PublicationCheckpoint::Renamed),
    PublicationEvent::CandidateSync(PublicationSync::Initial),
    PublicationEvent::ParentSync(PublicationSync::Initial),
    PublicationEvent::Checkpoint(PublicationCheckpoint::InitialSyncComplete),
    PublicationEvent::Checkpoint(PublicationCheckpoint::EntriesReopened),
    PublicationEvent::Checkpoint(PublicationCheckpoint::ContentVerified),
    PublicationEvent::Checkpoint(PublicationCheckpoint::ModeProtected),
    PublicationEvent::Checkpoint(PublicationCheckpoint::MarkerApplied),
    PublicationEvent::CandidateSync(PublicationSync::Commit),
    PublicationEvent::Checkpoint(PublicationCheckpoint::CandidateBarrierComplete),
    PublicationEvent::ParentSync(PublicationSync::Commit),
    PublicationEvent::Checkpoint(PublicationCheckpoint::DurabilityBarriersComplete),
    PublicationEvent::Checkpoint(PublicationCheckpoint::CommitMarked),
    PublicationEvent::Checkpoint(PublicationCheckpoint::FinalValidationComplete),
];

#[test]
fn durable_barriers_precede_the_commit_checkpoint() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let hooks = RecordingHooks::default();
    publish_with_hooks(root.path(), &path, &hooks).unwrap();
    let _cleanup = OutputCleanup::new(&path);
    assert_eq!(hooks.events.into_inner(), EXPECTED_PUBLICATION_ORDER);
}

#[test]
fn candidate_commit_barrier_failure_rolls_back() {
    assert_commit_barrier_failure(
        FailingBarrier::Candidate,
        INJECTED_CANDIDATE_COMMIT_SYNC_FAILURE,
    );
}

#[test]
fn parent_commit_barrier_failure_rolls_back() {
    assert_commit_barrier_failure(FailingBarrier::Parent, INJECTED_PARENT_COMMIT_SYNC_FAILURE);
}

fn assert_commit_barrier_failure(target: FailingBarrier, expected: &str) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let error = error_text(publish_with_hooks(root.path(), &path, &target));
    assert!(error.contains(expected), "{error}");
    assert!(!error.contains(COMMITTED_ACKNOWLEDGEMENT_ERROR), "{error}");
    assert!(!error.contains(COMMITTED_VALIDATION_ERROR), "{error}");
    assert!(!path.exists());
    assert_no_staging_name(root.path());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FailingBarrier {
    Candidate,
    Parent,
}

impl PublishHooks for FailingBarrier {
    fn sync_candidate(&self, phase: PublicationSync, candidate: &DirectoryHandle) -> Result<()> {
        if *self == Self::Candidate && phase == PublicationSync::Commit {
            assert_eq!(candidate.flags()?, libc::UF_IMMUTABLE);
            bail!(INJECTED_CANDIDATE_COMMIT_SYNC_FAILURE);
        }
        candidate.durability_barrier()
    }

    fn sync_parent(&self, phase: PublicationSync, parent: &DirectoryHandle) -> Result<()> {
        if *self == Self::Parent && phase == PublicationSync::Commit {
            bail!(INJECTED_PARENT_COMMIT_SYNC_FAILURE);
        }
        parent.durability_barrier()
    }
}

#[derive(Default)]
struct RecordingHooks {
    events: RefCell<Vec<PublicationEvent>>,
}

impl PublishHooks for RecordingHooks {
    fn sync_candidate(&self, phase: PublicationSync, candidate: &DirectoryHandle) -> Result<()> {
        if phase == PublicationSync::Commit {
            assert_eq!(candidate.flags()?, libc::UF_IMMUTABLE);
        }
        self.events
            .borrow_mut()
            .push(PublicationEvent::CandidateSync(phase));
        candidate.durability_barrier()
    }

    fn sync_parent(&self, phase: PublicationSync, parent: &DirectoryHandle) -> Result<()> {
        self.events
            .borrow_mut()
            .push(PublicationEvent::ParentSync(phase));
        parent.durability_barrier()
    }

    fn checkpoint(&self, checkpoint: PublicationCheckpoint) -> Result<()> {
        self.events
            .borrow_mut()
            .push(PublicationEvent::Checkpoint(checkpoint));
        Ok(())
    }
}
