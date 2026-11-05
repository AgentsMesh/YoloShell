use super::*;
use yolo_process_test_support::linux_containment::{
    LinuxContainmentRecord, LinuxContainmentRecovery, LinuxContainmentRecoveryPoll,
    LinuxContainmentSettlement, record_path,
};
use yolo_process_test_support::{ProcessIdentity, ResilienceProcessTree};

const CONTAINMENT_REQUIRED: &str = "runtime fixture has no retained Linux containment record";
const CONTAINMENT_INCOMPLETE: &str = "runtime fixture Linux containment is incomplete";

#[derive(Clone)]
pub(super) struct BoundContainment {
    record: LinuxContainmentRecord,
    file: RetainedJsonFileObservation,
}

pub(in super::super) struct LinuxRuntimeCleanup {
    record: LinuxContainmentRecord,
    root: ProcessIdentity,
    recovery: Option<LinuxContainmentRecovery>,
    settlement: Option<LinuxContainmentSettlement>,
}

impl LinuxRuntimeCleanup {
    pub(in super::super) fn poll(&mut self) -> Result<()> {
        if let Some(settlement) = &self.settlement {
            return settlement.verify().map_err(Into::into);
        }
        if self.recovery.is_none() {
            self.recovery = Some(LinuxContainmentRecovery::open(
                self.record.clone(),
                self.record.terminal_id,
                self.record.runtime_epoch,
                self.root,
            )?);
        }
        let recovery = self.recovery.as_mut().expect(CONTAINMENT_REQUIRED);
        if matches!(
            recovery.poll()?,
            LinuxContainmentRecoveryPoll::Settled { leaf_removed: true }
        ) {
            self.settlement = Some(
                self.recovery
                    .take()
                    .expect(CONTAINMENT_REQUIRED)
                    .into_settlement()?,
            );
        }
        Ok(())
    }

    pub(in super::super) fn is_settled(&self) -> bool {
        self.settlement.is_some()
    }
}

impl BoundContainment {
    pub(super) fn capture(paths: &Paths, runtime: &RuntimeRecord) -> Result<Option<Self>> {
        let path = record_path(
            &paths.session_dir(runtime.terminal_id),
            runtime.runtime_epoch,
        );
        if !path.try_exists()? {
            return Ok(None);
        }
        let (record, file) = RetainedJsonFileObservation::bind::<LinuxContainmentRecord>(&path)?;
        let root = super::super::cleanup_tree::runtime_child_identity(runtime);
        ensure!(
            record.validate_recovery(
                runtime.terminal_id,
                runtime.runtime_epoch,
                &runtime.boot_id,
                root,
            )?,
            CONTAINMENT_INCOMPLETE
        );
        Ok(Some(Self { record, file }))
    }

    pub(super) fn verify(&self) -> Result<()> {
        self.file.verify_unchanged()
    }
}

impl BoundRuntimeRecord {
    pub(in super::super) fn linux_cleanup_authority(&self) -> Option<LinuxRuntimeCleanup> {
        self.containment
            .as_ref()
            .map(|containment| LinuxRuntimeCleanup {
                record: containment.record.clone(),
                root: super::super::cleanup_tree::runtime_child_identity(&self.record),
                recovery: None,
                settlement: None,
            })
    }

    pub(in super::super) fn capture_cleanup_tree(&self) -> Result<ResilienceProcessTree> {
        if self.containment.is_some() {
            self.capture_linux_tree()
        } else {
            // Synthetic fixtures retain only their observed process closure.
            ResilienceProcessTree::capture(super::super::cleanup_tree::runtime_child_identity(
                &self.record,
            ))
        }
    }

    pub(in super::super) fn capture_linux_power_loss_tree(
        &self,
    ) -> Result<(
        ResilienceProcessTree,
        yolo_process_test_support::linux_containment::ResilienceContainmentRollback,
    )> {
        self.verify_runtime_file()?;
        let containment = self.containment.as_ref().context(CONTAINMENT_REQUIRED)?;
        let captured = ResilienceProcessTree::capture_linux_with_rollback(
            super::super::cleanup_tree::runtime_child_identity(&self.record),
            containment.record.clone(),
        )?;
        self.verify_runtime_file()?;
        Ok(captured)
    }

    pub(in super::super) fn capture_linux_tree(&self) -> Result<ResilienceProcessTree> {
        self.verify_runtime_file()?;
        let containment = self.containment.as_ref().context(CONTAINMENT_REQUIRED)?;
        let tree = ResilienceProcessTree::capture_linux(
            ProcessIdentity {
                pid: self.record.child_pid,
                start_time: self.record.child_start_time,
            },
            containment.record.clone(),
        )?;
        self.verify_runtime_file()?;
        Ok(tree)
    }
}
