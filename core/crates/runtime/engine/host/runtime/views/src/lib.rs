//! Runtime view and process-identity evidence shared by Host lifecycle paths.
//!
//! These predicates decide whether a runtime's recorded processes are proven
//! gone, and render the view a caller may show when its Worker is unreachable.

use anyhow::Result;
use yolo_terminal_model::{ExitRecord, RuntimeRecord, RuntimeView, TerminalRuntimeStatus};

pub mod runtime_loss {
    use yolo_terminal_model::RuntimeRecord;
    use yolo_process as process;

    pub fn runtime_record_identity_is_verifiable(
        record: &RuntimeRecord,
        terminal_id: uuid::Uuid,
    ) -> bool {
        record.schema_version == yolo_application_support::constants::RUNTIME_RECORD_SCHEMA_VERSION
            && record.terminal_id == terminal_id
            && record.worker_start_time != u64::default()
            && record.child_start_time != u64::default()
    }

    pub fn process_identities_proven_gone(record: &RuntimeRecord, boot: &str) -> bool {
        match process::boot_identity_relation(&record.boot_id, boot) {
            process::BootIdentityRelation::Different => true,
            process::BootIdentityRelation::Same => {
                crate::process_identity_is_gone(record.worker_pid, record.worker_start_time)
                    && crate::process_identity_is_gone(record.child_pid, record.child_start_time)
            }
            process::BootIdentityRelation::Unknown => false,
        }
    }

    pub fn identities_proven_gone(record: &RuntimeRecord, boot: &str) -> bool {
        record.schema_version == yolo_application_support::constants::RUNTIME_RECORD_SCHEMA_VERSION
            && !record.terminal_id.is_nil()
            && !record.runtime_epoch.is_nil()
            && !record.worker_id.is_nil()
            && record.worker_pid != u32::default()
            && record.child_pid != u32::default()
            && record.worker_start_time != u64::default()
            && record.child_start_time != u64::default()
            && process_identities_proven_gone(record, boot)
    }
}

pub fn unreachable_view_with_exit(
    record: &RuntimeRecord,
    exit: Option<ExitRecord>,
    boot: &str,
) -> RuntimeView {
    let exit = exit.filter(|exit| exit.runtime_epoch == record.runtime_epoch);
    RuntimeView {
        presentation: Default::default(),
        runtime_epoch: record.runtime_epoch,
        worker_id: record.worker_id,
        worker_pid: record.worker_pid,
        worker_start_time: record.worker_start_time,
        child_pid: record.child_pid,
        child_start_time: record.child_start_time,
        status: if exit.is_some() {
            TerminalRuntimeStatus::Exited
        } else if runtime_loss::identities_proven_gone(record, boot) {
            TerminalRuntimeStatus::Lost
        } else {
            TerminalRuntimeStatus::Unreachable
        },
        last_output_sequence: exit
            .as_ref()
            .map(|exit| exit.last_output_sequence)
            .unwrap_or(0),
        exit,
        bootstrap_delivery: record.bootstrap_delivery,
        capabilities: None,
        fault_reap: None,
    }
}

pub fn process_identity_is_gone(pid: u32, start_time: u64) -> bool {
    matches!(
        process_identity_evidence(
            yolo_process::try_process_identity_matches(pid, start_time),
            || yolo_process::is_process_alive(pid),
        ),
        ProcessIdentityEvidence::ProvenGone
    )
}

pub fn process_identity_is_verified_live(pid: u32, start_time: u64) -> bool {
    matches!(
        process_identity_evidence(
            yolo_process::try_process_identity_matches(pid, start_time),
            || yolo_process::is_process_alive(pid),
        ),
        ProcessIdentityEvidence::VerifiedLive
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessIdentityEvidence {
    VerifiedLive,
    ProvenGone,
    Indeterminate,
}

pub fn process_identity_evidence(
    identity: Result<bool>,
    process_is_alive: impl FnOnce() -> bool,
) -> ProcessIdentityEvidence {
    match identity {
        Ok(true) => ProcessIdentityEvidence::VerifiedLive,
        Ok(false) => ProcessIdentityEvidence::ProvenGone,
        Err(_) if process_is_alive() => ProcessIdentityEvidence::Indeterminate,
        Err(_) => ProcessIdentityEvidence::ProvenGone,
    }
}
