use super::{CodexSnapshotProof, SnapshotProofBasis, run_before_publication_hook};
use crate::agent::hooks::ReducerSnapshot;
use crate::codex_hooks::CodexSnapshotLineage;
use anyhow::{Context, Result, ensure};

impl CodexSnapshotProof {
    pub(crate) fn linearize_snapshot(&self, expected: &ReducerSnapshot) -> Result<()> {
        run_before_publication_hook();
        let loaded = self.load_expected()?;
        let loaded = serialize_snapshot(
            loaded
                .snapshot
                .as_ref()
                .context(super::super::constants::errors::SNAPSHOT_CORRUPT)?,
        )?;
        ensure!(
            loaded == serialize_snapshot(expected)?,
            super::super::constants::errors::SNAPSHOT_CORRUPT
        );
        Ok(())
    }

    pub(crate) fn lineage(&self) -> Result<CodexSnapshotLineage> {
        let loaded = self.load_expected()?;
        self.lock.authority().lineage(loaded.identity)
    }

    fn load_expected(&self) -> Result<super::super::snapshot_storage::LoadedSnapshot> {
        super::super::snapshot_storage::load_expected(
            &self.lock,
            &self.path,
            self.expected_identity(),
        )
    }

    fn expected_identity(&self) -> Option<crate::paths::identity::FileIdentity> {
        match self.basis {
            SnapshotProofBasis::AcquisitionBaseline(expected) => expected,
            SnapshotProofBasis::RetainedClaim(expected) => Some(expected),
        }
    }
}

fn serialize_snapshot(snapshot: &ReducerSnapshot) -> Result<Vec<u8>> {
    serde_json::to_vec(snapshot).context(super::super::constants::errors::SNAPSHOT_CORRUPT)
}
