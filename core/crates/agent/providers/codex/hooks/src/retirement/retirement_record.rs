use super::retirement_constants::*;
use super::{
    ACCEPTED_PROVIDER_VERSION, CodexAdmissionAuthority, CodexInvocationAuthority,
    CodexRetirementSealArtifact, CodexSnapshotLineage, CodexSnapshotProof,
};
use crate::agent::hooks::{HookIdentity, ProcessState, HookProviderKind, Reducer, ReducerSnapshot};
use crate::paths::identity::FileIdentity;
use crate::process::{BootIdentityRelation, ExactProcessGone};
use crate::sha256_digest::Sha256Digest;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[path = "retirement_record_support.rs"]
mod support;
use support::*;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexRetirementSeal {
    schema_version: u16,
    identity: HookIdentity,
    boot: crate::process::boot_identity::BootIdentity,
    admission_sha256: Sha256Digest,
    admission_file_identity: FileIdentity,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexRetirementDisposition {
    CleanFinal,
    Abandoned,
    Revoked,
    AuthorityExhausted,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CodexRetirementRecord {
    schema_version: u16,
    seal: CodexRetirementSeal,
    seal_sha256: Sha256Digest,
    seal_file_identity: FileIdentity,
    snapshot_sha256: Sha256Digest,
    snapshot_lineage: CodexSnapshotLineage,
    authority_revision: u64,
    last_sequence: u64,
    disposition: CodexRetirementDisposition,
}

impl CodexRetirementSeal {
    pub fn capture(admission: &CodexAdmissionAuthority, gone: ExactProcessGone) -> Result<Self> {
        admission.verify().map_err(|_| corrupt())?;
        let identity = admission.record().request().identity.clone();
        ensure!(gone.identity() == process_identity(&identity), corrupt());
        ensure!(gone.boot_identity().known(), corrupt());
        let seal = Self {
            schema_version: SEAL_SCHEMA_VERSION,
            identity,
            boot: gone.boot_identity(),
            admission_sha256: admission_digest(admission)?,
            admission_file_identity: admission.file_identity(),
        };
        seal.validate_for(admission)?;
        Ok(seal)
    }

    pub(crate) fn validate_for(&self, admission: &CodexAdmissionAuthority) -> Result<()> {
        self.identity.validate().map_err(|_| corrupt())?;
        admission.verify().map_err(|_| corrupt())?;
        admission
            .record()
            .validate_for(&self.identity)
            .map_err(|_| corrupt())?;
        ensure!(self.schema_version == SEAL_SCHEMA_VERSION, corrupt());
        ensure!(self.identity.process.is_valid(), corrupt());
        ensure!(self.boot.known(), corrupt());
        ensure!(
            self.boot.relation_to_current() != BootIdentityRelation::Unknown,
            corrupt()
        );
        ensure!(
            self.admission_sha256 == admission_digest(admission)?,
            corrupt()
        );
        ensure!(
            self.admission_file_identity == admission.file_identity(),
            corrupt()
        );
        Ok(())
    }

    pub fn identity(&self) -> &HookIdentity {
        &self.identity
    }
}

impl CodexRetirementRecord {
    pub fn capture(
        seal: &CodexRetirementSealArtifact,
        reducer: &Reducer,
        admission: &CodexAdmissionAuthority,
        proof: &CodexSnapshotProof,
    ) -> Result<Self> {
        let snapshot = reducer.snapshot().map_err(|_| corrupt())?;
        let state = reducer.state();
        let disposition = disposition(reducer)?;
        let record = Self {
            schema_version: RECORD_SCHEMA_VERSION,
            seal_sha256: seal_digest(seal.record())?,
            seal_file_identity: seal.file_identity(),
            snapshot_sha256: snapshot_digest(&snapshot)?,
            snapshot_lineage: proof.lineage().map_err(|_| corrupt())?,
            authority_revision: reducer.revision(),
            last_sequence: state.last_sequence.0,
            disposition,
            seal: seal.record().clone(),
        };
        record.validate_for(&snapshot, admission, seal, proof)?;
        Ok(record)
    }

    pub(crate) fn validate_for(
        &self,
        snapshot: &ReducerSnapshot,
        admission: &CodexAdmissionAuthority,
        seal: &CodexRetirementSealArtifact,
        proof: &CodexSnapshotProof,
    ) -> Result<()> {
        ensure!(self.schema_version == RECORD_SCHEMA_VERSION, corrupt());
        ensure!(&self.seal == seal.record(), corrupt());
        self.seal.validate_for(admission)?;
        ensure!(self.seal_sha256 == seal_digest(&self.seal)?, corrupt());
        ensure!(self.seal_file_identity == seal.file_identity(), corrupt());
        ensure!(
            self.snapshot_sha256 == snapshot_digest(snapshot)?,
            corrupt()
        );
        proof.linearize_snapshot(snapshot).map_err(|_| corrupt())?;
        ensure!(
            self.snapshot_lineage == proof.lineage().map_err(|_| corrupt())?,
            corrupt()
        );
        let reducer = Reducer::restore(
            snapshot.clone(),
            self.seal.identity.clone(),
            HookProviderKind::Codex,
            ACCEPTED_PROVIDER_VERSION,
        )
        .map_err(|_| corrupt())?;
        let state = reducer.state();
        let disposition = disposition(&reducer)?;
        ensure!(self.authority_revision == reducer.revision(), corrupt());
        ensure!(self.last_sequence == state.last_sequence.0, corrupt());
        ensure!(self.disposition == disposition, corrupt());
        Ok(())
    }

    pub(crate) fn seal(&self) -> &CodexRetirementSeal {
        &self.seal
    }

    pub(crate) fn disposition(&self) -> CodexRetirementDisposition {
        self.disposition
    }

    pub(crate) fn require_invocation(&self, authority: CodexInvocationAuthority) -> Result<()> {
        self.snapshot_lineage
            .require_invocation(authority)
            .map_err(|_| corrupt())
    }
}
