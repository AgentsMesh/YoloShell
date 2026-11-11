use super::runtime_record_validation::{validate_runtime_record, validate_runtime_record_for_boot};
use super::*;
use anyhow::{Context, Result, ensure};
use yolo_runtime::paths::RetainedJsonFileObservation;

#[cfg(target_os = "linux")]
#[path = "runtime_record_containment.rs"]
mod containment;
#[cfg(target_os = "linux")]
pub(super) use containment::LinuxRuntimeCleanup;

const RUNTIME_VIEW_MISSING_DETAIL: &str = "trusted session view has no runtime";
const RUNTIME_VIEW_MISMATCH_DETAIL: &str =
    "runtime cleanup record does not match the trusted runtime view";
const RUNTIME_VIEW_STATUS_UNTRUSTED_DETAIL: &str =
    "runtime cleanup view was not authenticated from a live worker";
const RUNTIME_GENERATION_ROLLBACK_DETAIL: &str =
    "runtime cleanup record rolled back its controller generation";
const RUNTIME_FILE_MISMATCH_DETAIL: &str =
    "runtime cleanup record does not match its retained durable file";
#[derive(Clone)]
pub(super) struct BoundRuntimeRecord {
    record: RuntimeRecord,
    runtime_file: RetainedJsonFileObservation,
    #[cfg(target_os = "linux")]
    containment: Option<containment::BoundContainment>,
}

impl BoundRuntimeRecord {
    pub(super) fn bind_view(
        paths: &Paths,
        record: RuntimeRecord,
        view: &yolo_runtime::TerminalView,
    ) -> Result<Self> {
        let trusted = view.runtime.as_ref().context(RUNTIME_VIEW_MISSING_DETAIL)?;
        Self::bind_runtime(paths, record, view.manifest.terminal_id, trusted)
    }

    pub(super) fn bind_runtime(
        paths: &Paths,
        record: RuntimeRecord,
        terminal_id: Uuid,
        trusted: &yolo_runtime::model::RuntimeView,
    ) -> Result<Self> {
        validate_runtime_record(paths, &record, Some(terminal_id))?;
        Self::bind_validated(paths, record, terminal_id, trusted)
    }

    pub(super) fn bind_view_for_boot(
        paths: &Paths,
        record: RuntimeRecord,
        view: &yolo_runtime::TerminalView,
        expected_boot_id: &str,
    ) -> Result<Self> {
        let trusted = view.runtime.as_ref().context(RUNTIME_VIEW_MISSING_DETAIL)?;
        validate_runtime_record_for_boot(
            paths,
            &record,
            Some(view.manifest.terminal_id),
            expected_boot_id,
        )?;
        Self::bind_validated(paths, record, view.manifest.terminal_id, trusted)
    }

    pub(super) fn record(&self) -> &RuntimeRecord {
        &self.record
    }

    pub(super) fn verify_runtime_file(&self) -> Result<()> {
        self.runtime_file.verify_unchanged()?;
        #[cfg(target_os = "linux")]
        if let Some(containment) = &self.containment {
            containment.verify()?;
        }
        Ok(())
    }

    pub(super) fn refresh(&self, candidate: &RuntimeRecord) -> Result<Self> {
        self.validate_refresh(candidate)?;
        let runtime_file = refresh_runtime_file(&self.runtime_file, candidate)?;
        Ok(Self {
            record: candidate.clone(),
            runtime_file,
            #[cfg(target_os = "linux")]
            containment: self.containment.clone(),
        })
    }

    pub(super) fn refresh_current(&self) -> Result<Self> {
        let (record, runtime_file) = self.runtime_file.refresh::<RuntimeRecord>()?;
        self.validate_refresh(&record)?;
        Ok(Self {
            record,
            runtime_file,
            #[cfg(target_os = "linux")]
            containment: self.containment.clone(),
        })
    }

    pub(super) fn require_view_matches(&self, view: &yolo_runtime::TerminalView) -> Result<()> {
        let trusted = view.runtime.as_ref().context(RUNTIME_VIEW_MISSING_DETAIL)?;
        ensure_view_matches(&self.record, view.manifest.terminal_id, trusted)
    }

    fn validate_refresh(&self, candidate: &RuntimeRecord) -> Result<()> {
        ensure!(
            same_cleanup_identity(&self.record, candidate),
            RUNTIME_VIEW_MISMATCH_DETAIL
        );
        ensure!(
            candidate.controller_generation >= self.record.controller_generation,
            RUNTIME_GENERATION_ROLLBACK_DETAIL
        );
        Ok(())
    }

    pub(super) fn refresh_view(
        &self,
        candidate: &RuntimeRecord,
        view: &yolo_runtime::TerminalView,
    ) -> Result<Self> {
        let trusted = view.runtime.as_ref().context(RUNTIME_VIEW_MISSING_DETAIL)?;
        ensure_view_matches(candidate, view.manifest.terminal_id, trusted)?;
        self.refresh(candidate)
    }

    fn bind_validated(
        paths: &Paths,
        record: RuntimeRecord,
        terminal_id: Uuid,
        trusted: &yolo_runtime::model::RuntimeView,
    ) -> Result<Self> {
        ensure!(
            matches!(
                trusted.status,
                TerminalRuntimeStatus::Starting
                    | TerminalRuntimeStatus::Running
                    | TerminalRuntimeStatus::Finalizing
                    | TerminalRuntimeStatus::OutputBlocked
            ),
            RUNTIME_VIEW_STATUS_UNTRUSTED_DETAIL
        );
        ensure_view_matches(&record, terminal_id, trusted)?;
        let runtime_file = bind_runtime_file(paths, &record)?;
        #[cfg(target_os = "linux")]
        let containment = containment::BoundContainment::capture(paths, &record)?;
        Ok(Self {
            record,
            runtime_file,
            #[cfg(target_os = "linux")]
            containment,
        })
    }
}

fn bind_runtime_file(paths: &Paths, record: &RuntimeRecord) -> Result<RetainedJsonFileObservation> {
    let (durable, observation) = RetainedJsonFileObservation::bind::<RuntimeRecord>(
        &paths.runtime_record(record.terminal_id),
    )?;
    ensure!(durable == *record, RUNTIME_FILE_MISMATCH_DETAIL);
    Ok(observation)
}

fn refresh_runtime_file(
    current: &RetainedJsonFileObservation,
    record: &RuntimeRecord,
) -> Result<RetainedJsonFileObservation> {
    let (durable, observation) = current.refresh::<RuntimeRecord>()?;
    ensure!(durable == *record, RUNTIME_FILE_MISMATCH_DETAIL);
    Ok(observation)
}

pub(super) fn ensure_view_matches(
    record: &RuntimeRecord,
    terminal_id: Uuid,
    trusted: &yolo_runtime::model::RuntimeView,
) -> Result<()> {
    ensure!(
        terminal_id == record.terminal_id
            && trusted.runtime_epoch == record.runtime_epoch
            && trusted.worker_id == record.worker_id
            && trusted.worker_pid == record.worker_pid
            && trusted.worker_start_time == record.worker_start_time
            && trusted.child_pid == record.child_pid
            && trusted.child_start_time == record.child_start_time
            && trusted.bootstrap_delivery == record.bootstrap_delivery,
        RUNTIME_VIEW_MISMATCH_DETAIL
    );
    Ok(())
}

fn same_cleanup_identity(left: &RuntimeRecord, right: &RuntimeRecord) -> bool {
    left.has_same_process_identity(right)
        && left.started_at_ms == right.started_at_ms
        && left.command == right.command
        && left.args == right.args
        && left.cwd == right.cwd
}
