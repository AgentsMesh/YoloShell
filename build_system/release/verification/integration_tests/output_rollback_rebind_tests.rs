use super::support::error_text;
use super::test_constants::{
    INJECTED_PUBLICATION_FAILURE, OUTPUT_REBOUND_NAME, OUTPUT_TEST_DESTINATION_NAME,
};
use crate::output_publish::{PublicationCheckpoint, PublicationSync, PublishHooks};
use crate::output_stage_commit::ROLLBACK_AUTHORITY_ERROR;
use crate::test_support::{assert_no_staging_name, publish_with_hooks, unseal};
use anyhow::{Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use yolo_release_contracts::OUTPUT_FILE_NAMES;
use yolo_release_filesystem::DirectoryHandle;

const REPLACEMENT_ENTRY: &str = "replacement-sentinel";
const REPLACEMENT_BYTES: &[u8] = b"preserve concurrent replacement";
const REBIND_BOUNDARIES: &[RebindBoundary] = &[
    RebindBoundary::Checkpoint(PublicationCheckpoint::Renamed),
    RebindBoundary::CandidateSync(PublicationSync::Initial),
    RebindBoundary::ParentSync(PublicationSync::Initial),
    RebindBoundary::Checkpoint(PublicationCheckpoint::InitialSyncComplete),
    RebindBoundary::Checkpoint(PublicationCheckpoint::EntriesReopened),
    RebindBoundary::Checkpoint(PublicationCheckpoint::ContentVerified),
    RebindBoundary::Checkpoint(PublicationCheckpoint::ModeProtected),
    RebindBoundary::Checkpoint(PublicationCheckpoint::MarkerApplied),
    RebindBoundary::CandidateSync(PublicationSync::Commit),
    RebindBoundary::Checkpoint(PublicationCheckpoint::CandidateBarrierComplete),
    RebindBoundary::ParentSync(PublicationSync::Commit),
];

#[test]
fn rollback_never_moves_or_deletes_a_rebound_destination() {
    for boundary in REBIND_BOUNDARIES {
        assert_rebind_is_preserved(*boundary);
    }
}

fn assert_rebind_is_preserved(boundary: RebindBoundary) {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(OUTPUT_TEST_DESTINATION_NAME);
    let rebound = root.path().join(OUTPUT_REBOUND_NAME);
    let hooks = RebindAndFail {
        boundary,
        path: path.clone(),
        rebound: rebound.clone(),
    };

    let error = error_text(publish_with_hooks(root.path(), &path, &hooks));

    assert!(error.contains(INJECTED_PUBLICATION_FAILURE), "{error}");
    assert!(error.contains(ROLLBACK_AUTHORITY_ERROR), "{error}");
    assert_eq!(
        fs::read(path.join(REPLACEMENT_ENTRY)).unwrap(),
        REPLACEMENT_BYTES
    );
    assert!(rebound.is_dir());
    for name in OUTPUT_FILE_NAMES {
        assert!(!rebound.join(name).exists(), "{boundary:?}: {name}");
    }
    assert_no_staging_name(root.path());
}

struct RebindAndFail {
    boundary: RebindBoundary,
    path: PathBuf,
    rebound: PathBuf,
}

impl PublishHooks for RebindAndFail {
    fn checkpoint(&self, checkpoint: PublicationCheckpoint) -> Result<()> {
        self.trigger(RebindBoundary::Checkpoint(checkpoint))
    }

    fn sync_candidate(&self, phase: PublicationSync, candidate: &DirectoryHandle) -> Result<()> {
        self.trigger(RebindBoundary::CandidateSync(phase))?;
        candidate.durability_barrier()
    }

    fn sync_parent(&self, phase: PublicationSync, parent: &DirectoryHandle) -> Result<()> {
        self.trigger(RebindBoundary::ParentSync(phase))?;
        parent.durability_barrier()
    }
}

impl RebindAndFail {
    fn trigger(&self, boundary: RebindBoundary) -> Result<()> {
        if boundary == self.boundary {
            rebind(&self.path, &self.rebound)?;
            bail!(INJECTED_PUBLICATION_FAILURE);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RebindBoundary {
    Checkpoint(PublicationCheckpoint),
    CandidateSync(PublicationSync),
    ParentSync(PublicationSync),
}

fn rebind(path: &Path, rebound: &Path) -> Result<()> {
    unseal(path)?;
    fs::rename(path, rebound)?;
    fs::create_dir(path)?;
    fs::write(path.join(REPLACEMENT_ENTRY), REPLACEMENT_BYTES)?;
    Ok(())
}
