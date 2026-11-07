use crate::journal::{Journal, RetainedJournalEvidence};
use crate::model::RuntimeRecord;
use crate::paths::Paths;
use crate::paths::identity::FileIdentity;
use crate::process::{MacosSecondaryCustody, ProcessIdentity, ProcessIdentityObservation};
use crate::pty_custody::{CustodyDurableLease, CustodyRuntimeIdentity};
use anyhow::{Context, Result, ensure};
use yolo_paths::durable_mutation::SessionDurableMutationGuard;

const RUNTIME_DETAIL: &str = "durable PTY custody retirement runtime evidence changed";
const JOURNAL_DETAIL: &str = "durable PTY custody retirement journal evidence changed";
const PROCESS_DETAIL: &str = "durable PTY custody retirement process evidence is not exact";

pub(super) struct RetirementProof {
    runtime: RuntimeRecord,
    runtime_identity: FileIdentity,
    journal: RetainedJournalEvidence,
    controller_generation: u64,
    recovery_identity: crate::pty_custody::sha256_digest::Sha256Digest,
}

impl RetirementProof {
    pub(super) fn runtime(&self) -> &RuntimeRecord {
        &self.runtime
    }

    pub(super) fn capture(
        guard: &SessionDurableMutationGuard<'_>,
        paths: &Paths,
        custody: &MacosSecondaryCustody,
        expected_runtime: &CustodyRuntimeIdentity,
        expected_lease: CustodyDurableLease,
    ) -> Result<Self> {
        guard.require_no_pending_intent()?;
        let runtime_path = paths.runtime_record(guard.terminal_id());
        let (runtime, runtime_identity) =
            guard.namespace().read_json_with_identity(&runtime_path)?;
        validate_runtime(&runtime, expected_runtime, expected_lease)?;
        ensure!(
            custody.recovery_identity_sha256() == expected_lease.recovery_identity_sha256,
            PROCESS_DETAIL
        );
        validate_processes(&runtime)?;
        custody.confirm_root_exit()?;
        ensure!(custody.closure_settled()?, PROCESS_DETAIL);
        let journal = Journal::inspect_retained_evidence(
            guard.namespace().file(&paths.journal(guard.terminal_id()))?,
            runtime.runtime_epoch,
        )
        .context(JOURNAL_DETAIL)?;
        Ok(Self {
            runtime,
            runtime_identity,
            journal,
            controller_generation: expected_lease.controller_generation,
            recovery_identity: expected_lease.recovery_identity_sha256,
        })
    }

    pub(super) fn evidence(&self) -> crate::pty_custody::CustodyRetirementEvidence {
        self.journal
            .custody_retirement_evidence(self.controller_generation, self.runtime_identity)
    }

    pub(super) fn verify(
        &self,
        guard: &SessionDurableMutationGuard<'_>,
        paths: &Paths,
        custody: &MacosSecondaryCustody,
    ) -> Result<()> {
        guard.require_no_pending_intent()?;
        let (runtime, identity): (RuntimeRecord, FileIdentity) = guard
            .namespace()
            .read_json_with_identity(&paths.runtime_record(guard.terminal_id()))?;
        ensure!(
            runtime == self.runtime && identity == self.runtime_identity,
            RUNTIME_DETAIL
        );
        validate_processes(&runtime)?;
        ensure!(
            custody.recovery_identity_sha256() == self.recovery_identity,
            PROCESS_DETAIL
        );
        custody.confirm_root_exit()?;
        ensure!(custody.closure_settled()?, PROCESS_DETAIL);
        self.journal.verify_current().context(JOURNAL_DETAIL)
    }
}

fn validate_runtime(
    runtime: &RuntimeRecord,
    expected: &CustodyRuntimeIdentity,
    lease: CustodyDurableLease,
) -> Result<()> {
    let claim_is_exact = lease
        .guardian
        .is_some_and(|binding| binding.matches_durable_lease(expected, &lease));
    ensure!(
        expected.matches_record(runtime)
            && runtime.controller_generation == lease.controller_generation
            && claim_is_exact,
        RUNTIME_DETAIL
    );
    Ok(())
}

fn validate_processes(runtime: &RuntimeRecord) -> Result<()> {
    ensure!(
        crate::process::boot_identity_relation(&runtime.boot_id, &crate::process::boot_id())
            == crate::process::BootIdentityRelation::Same,
        PROCESS_DETAIL
    );
    for process in [
        ProcessIdentity {
            pid: runtime.worker_pid,
            start_time: runtime.worker_start_time,
        },
        ProcessIdentity {
            pid: runtime.child_pid,
            start_time: runtime.child_start_time,
        },
    ] {
        ensure!(
            process.observe() == ProcessIdentityObservation::Gone,
            PROCESS_DETAIL
        );
    }
    Ok(())
}
