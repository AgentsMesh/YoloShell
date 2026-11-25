use super::proof::RetirementProof;
use crate::paths::{Paths, identity::FileIdentity};
use crate::process::MacosSecondaryCustody;
use crate::pty_custody::CustodyDurableRecord;
use anyhow::{Result, ensure};
use std::path::PathBuf;
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const RETIREMENT_AUTHORITY_CHANGED: &str = "runtime retirement settlement authority changed";

/// Borrowed authority for one exact runtime while its PTY closure and durable evidence remain pinned.
pub struct VerifiedRuntimeRetirement<'a, 'namespace> {
    guard: &'a SessionDurableMutationGuard<'namespace>,
    paths: &'a Paths,
    custody: &'a MacosSecondaryCustody,
    proof: &'a RetirementProof,
    predecessor: &'a CustodyDurableRecord,
    predecessor_identity: FileIdentity,
}

impl<'a, 'namespace> VerifiedRuntimeRetirement<'a, 'namespace> {
    pub(super) fn new(
        guard: &'a SessionDurableMutationGuard<'namespace>,
        paths: &'a Paths,
        custody: &'a MacosSecondaryCustody,
        proof: &'a RetirementProof,
        predecessor: &'a CustodyDurableRecord,
        predecessor_identity: FileIdentity,
    ) -> Result<Self> {
        let authority = Self {
            guard,
            paths,
            custody,
            proof,
            predecessor,
            predecessor_identity,
        };
        authority.verify()?;
        Ok(authority)
    }

    pub fn verify(&self) -> Result<()> {
        self.proof.verify(self.guard, self.paths, self.custody)?;
        let current: (CustodyDurableRecord, FileIdentity) = self
            .guard
            .namespace()
            .read_json_with_identity(&self.paths.pty_custody(self.guard.terminal_id()))?;
        ensure!(
            current == (self.predecessor.clone(), self.predecessor_identity),
            RETIREMENT_AUTHORITY_CHANGED
        );
        self.proof.verify(self.guard, self.paths, self.custody)
    }

    pub(super) fn settle(self, settle: impl FnOnce(&Self) -> Result<()>) -> Result<()> {
        self.verify()?;
        settle(&self)?;
        self.verify()
    }

    pub fn mutation_guard(&self) -> &SessionDurableMutationGuard<'namespace> {
        self.guard
    }

    pub fn session_directory(&self) -> PathBuf {
        self.paths.session_dir(self.guard.terminal_id())
    }

    pub fn worker(&self) -> crate::process::ProcessIdentity {
        let runtime = self.proof.runtime();
        crate::process::ProcessIdentity {
            pid: runtime.worker_pid,
            start_time: runtime.worker_start_time,
        }
    }

    pub fn boot_id(&self) -> &str {
        &self.proof.runtime().boot_id
    }
}
