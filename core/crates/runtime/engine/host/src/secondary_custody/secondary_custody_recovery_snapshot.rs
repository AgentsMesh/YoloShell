use crate::host::{Host, RuntimeRecord};
use crate::journal::{Journal, RetainedJournalEvidence};
use crate::paths::identity::FileIdentity;
use crate::pty_custody::{CustodyDurableRecord, CustodyRuntimeIdentity};
use anyhow::{Result, ensure};

const RUNTIME_DETAIL: &str = "durable PTY custody retirement runtime changed";
const RECEIPT_DETAIL: &str = "durable PTY custody retirement receipt is not exact";

pub(in crate::host) struct RecoverySnapshot {
    pub(in crate::host) runtime: RuntimeRecord,
    pub(in crate::host) runtime_identity: FileIdentity,
    pub(in crate::host) custody: CustodyDurableRecord,
    pub(in crate::host) custody_identity: FileIdentity,
    pub(in crate::host) journal: RetainedJournalEvidence,
}

impl RecoverySnapshot {
    pub(in crate::host) fn capture(host: &Host, expected: &CustodyRuntimeIdentity) -> Result<Self> {
        let runtime_path = host.paths.runtime_record(expected.terminal_id);
        let (runtime, runtime_identity) = host.namespace.read_json_with_identity(&runtime_path)?;
        ensure!(expected.matches_record(&runtime), RUNTIME_DETAIL);
        let custody_path = host.paths.pty_custody(expected.terminal_id);
        let (custody, custody_identity) = host.namespace.read_json_with_identity(&custody_path)?;
        let journal = Journal::inspect_retained_evidence(
            host.namespace
                .file(&host.paths.journal(expected.terminal_id))?,
            runtime.runtime_epoch,
        )?;
        let snapshot = Self {
            runtime,
            runtime_identity,
            custody,
            custody_identity,
            journal,
        };
        snapshot.validate_lease(host)?;
        snapshot.verify(host)?;
        Ok(snapshot)
    }

    pub(in crate::host) fn verify(&self, host: &Host) -> Result<()> {
        self.verify_source(host)?;
        self.verify_custody(host)
    }

    pub(in crate::host) fn verify_source(&self, host: &Host) -> Result<()> {
        let runtime_path = host.paths.runtime_record(self.runtime.terminal_id);
        let current: (RuntimeRecord, FileIdentity) =
            host.namespace.read_json_with_identity(&runtime_path)?;
        ensure!(
            current == (self.runtime.clone(), self.runtime_identity),
            RUNTIME_DETAIL
        );
        self.validate_lease(host)?;
        self.journal.verify_current().map_err(Into::into)
    }

    pub(in crate::host) fn verify_custody(&self, host: &Host) -> Result<()> {
        let custody_path = host.paths.pty_custody(self.runtime.terminal_id);
        let current: (CustodyDurableRecord, FileIdentity) =
            host.namespace.read_json_with_identity(&custody_path)?;
        ensure!(
            current == (self.custody.clone(), self.custody_identity),
            RECEIPT_DETAIL
        );
        Ok(())
    }
}
