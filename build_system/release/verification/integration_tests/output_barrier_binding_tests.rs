use super::support::error_text;
use super::test_constants::{
    OUTPUT_TEST_DESTINATION_NAME, OUTPUT_TRANSIENT_NAME, TRANSIENT_PARENT_BYTES,
};
use crate::output_publish::{PublicationSync, PublishHooks};
use crate::test_support::{
    OutputCleanup, assert_no_staging_name, publish_with_hooks, seal, unseal,
};
use crate::verify::{VerifyRequest, verify};
use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use yolo_release_contracts::{COMMITTED_ACKNOWLEDGEMENT_ERROR, COMMITTED_VALIDATION_ERROR};
use yolo_release_filesystem::DirectoryHandle;

#[test]
fn restored_candidate_state_cannot_escape_the_captured_barrier_seal() {
    assert_restored_state_is_uncertain(RestoredState::Candidate);
}

#[test]
fn restored_parent_state_cannot_escape_the_captured_barrier_seal() {
    assert_restored_state_is_uncertain(RestoredState::Parent);
}

fn assert_restored_state_is_uncertain(state: RestoredState) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let hooks = RestoreDuringParentBarrier {
        state,
        candidate: path.clone(),
    };
    let error = error_text(publish_with_hooks(root.path(), &path, &hooks));
    let _cleanup = OutputCleanup::new(&path);
    assert!(error.contains(COMMITTED_VALIDATION_ERROR), "{error}");
    assert!(!error.contains(COMMITTED_ACKNOWLEDGEMENT_ERROR), "{error}");
    verify(VerifyRequest {
        output_directory: &path,
    })
    .unwrap();
    assert_no_staging_name(root.path());
}

#[derive(Clone, Copy)]
enum RestoredState {
    Candidate,
    Parent,
}

struct RestoreDuringParentBarrier {
    state: RestoredState,
    candidate: PathBuf,
}

impl PublishHooks for RestoreDuringParentBarrier {
    fn sync_parent(&self, phase: PublicationSync, parent: &DirectoryHandle) -> Result<()> {
        if phase == PublicationSync::Commit {
            match self.state {
                RestoredState::Candidate => {
                    unseal(&self.candidate)?;
                    seal(&self.candidate)?;
                }
                RestoredState::Parent => {
                    let transient = parent.display_path().join(OUTPUT_TRANSIENT_NAME);
                    fs::write(&transient, TRANSIENT_PARENT_BYTES)?;
                    fs::remove_file(transient)?;
                }
            }
        }
        parent.durability_barrier()
    }
}
