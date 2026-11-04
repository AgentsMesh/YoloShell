use super::support::ARCHIVE_BYTES;
use super::test_constants::{
    CANDIDATE_BARRIER_CHECKPOINT, COMMIT_MARKED_CHECKPOINT, CONTENT_VERIFIED_CHECKPOINT,
    DURABILITY_BARRIERS_CHECKPOINT, ENTRIES_REOPENED_CHECKPOINT, FINAL_VALIDATION_CHECKPOINT,
    INITIAL_SYNC_CHECKPOINT, MARKER_APPLIED_CHECKPOINT, MODE_PROTECTED_CHECKPOINT,
    OUTPUT_TEST_DESTINATION_NAME, PUBLICATION_CRASH_CHECKPOINT_ENVIRONMENT,
    PUBLICATION_CRASH_ROOT_ENVIRONMENT, PUBLICATION_CRASH_TEST_FILTER, RENAMED_CHECKPOINT,
    TEST_EXACT_ARGUMENT, TEST_SUCCESS_EXIT_STATUS,
};
use crate::output::OutputTransaction;
use crate::output_publish::{PublicationCheckpoint, PublishHooks};
use crate::test_support::{OutputCleanup, evidence};
use crate::verify::{VerifyRequest, verify};
use anyhow::Result;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::process::Command;
use yolo_release_candidate_artifact::PinnedArtifact;
use yolo_release_contracts::OUTPUT_FILE_NAMES;

#[derive(Clone, Copy)]
enum VerificationOutcome {
    Accept,
    Reject,
}

const CRASH_CASES: &[(PublicationCheckpoint, &str, VerificationOutcome)] = &[
    (
        PublicationCheckpoint::Renamed,
        RENAMED_CHECKPOINT,
        VerificationOutcome::Reject,
    ),
    (
        PublicationCheckpoint::InitialSyncComplete,
        INITIAL_SYNC_CHECKPOINT,
        VerificationOutcome::Reject,
    ),
    (
        PublicationCheckpoint::EntriesReopened,
        ENTRIES_REOPENED_CHECKPOINT,
        VerificationOutcome::Reject,
    ),
    (
        PublicationCheckpoint::ContentVerified,
        CONTENT_VERIFIED_CHECKPOINT,
        VerificationOutcome::Reject,
    ),
    (
        PublicationCheckpoint::ModeProtected,
        MODE_PROTECTED_CHECKPOINT,
        VerificationOutcome::Reject,
    ),
    (
        PublicationCheckpoint::MarkerApplied,
        MARKER_APPLIED_CHECKPOINT,
        VerificationOutcome::Accept,
    ),
    (
        PublicationCheckpoint::CandidateBarrierComplete,
        CANDIDATE_BARRIER_CHECKPOINT,
        VerificationOutcome::Accept,
    ),
    (
        PublicationCheckpoint::DurabilityBarriersComplete,
        DURABILITY_BARRIERS_CHECKPOINT,
        VerificationOutcome::Accept,
    ),
    (
        PublicationCheckpoint::CommitMarked,
        COMMIT_MARKED_CHECKPOINT,
        VerificationOutcome::Accept,
    ),
    (
        PublicationCheckpoint::FinalValidationComplete,
        FINAL_VALIDATION_CHECKPOINT,
        VerificationOutcome::Accept,
    ),
];

#[test]
fn publication_crash_checkpoints_distinguish_marker_durability() {
    if let Some(root) = std::env::var_os(PUBLICATION_CRASH_ROOT_ENVIRONMENT) {
        run_crash_fixture(Path::new(&root), selected_checkpoint());
        panic!("publication completed without reaching the selected crash checkpoint");
    }
    for &(_, name, outcome) in CRASH_CASES {
        assert_crash_case(name, outcome);
    }
}

fn assert_crash_case(checkpoint: &str, outcome: VerificationOutcome) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let status = Command::new(std::env::current_exe().unwrap())
        .args([TEST_EXACT_ARGUMENT, PUBLICATION_CRASH_TEST_FILTER])
        .env(PUBLICATION_CRASH_ROOT_ENVIRONMENT, root.path())
        .env(PUBLICATION_CRASH_CHECKPOINT_ENVIRONMENT, checkpoint)
        .status()
        .unwrap();
    assert!(status.success(), "crash fixture failed at {checkpoint}");
    assert!(path.is_dir(), "formal name absent at {checkpoint}");
    let _cleanup = OutputCleanup::new(&path);
    let result = verify(VerifyRequest {
        output_directory: &path,
    });
    let accepted = matches!(outcome, VerificationOutcome::Accept);
    assert_eq!(result.is_ok(), accepted, "checkpoint {checkpoint}");
    if accepted {
        assert_exact_entries(&path);
    }
}

fn run_crash_fixture(root: &Path, checkpoint: PublicationCheckpoint) {
    let path = root.join(OUTPUT_TEST_DESTINATION_NAME);
    let artifact = PinnedArtifact::fixture(root, ARCHIVE_BYTES).unwrap();
    let (checksum, provenance) = evidence();
    OutputTransaction::prepare(&path)
        .unwrap()
        .publish_with_hooks(&artifact, &checksum, &provenance, &CrashAt(checkpoint))
        .unwrap();
}

struct CrashAt(PublicationCheckpoint);

impl PublishHooks for CrashAt {
    fn checkpoint(&self, checkpoint: PublicationCheckpoint) -> Result<()> {
        if checkpoint == self.0 {
            unsafe { libc::_exit(TEST_SUCCESS_EXIT_STATUS) };
        }
        Ok(())
    }
}

fn selected_checkpoint() -> PublicationCheckpoint {
    let selected = std::env::var(PUBLICATION_CRASH_CHECKPOINT_ENVIRONMENT).unwrap();
    CRASH_CASES
        .iter()
        .find(|(_, name, _)| *name == selected)
        .map(|(checkpoint, _, _)| *checkpoint)
        .unwrap_or_else(|| panic!("unknown publication crash checkpoint: {selected}"))
}

fn assert_exact_entries(path: &Path) {
    let actual = fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect::<BTreeSet<_>>();
    let expected = OUTPUT_FILE_NAMES
        .iter()
        .map(ToString::to_string)
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
}
