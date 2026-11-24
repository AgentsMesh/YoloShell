use super::StartupCheckpointEvidence;
use anyhow::Result;
use uuid::Uuid;
use yolo_terminal_model::RuntimeRecord;

pub fn source_before_reboot_migration(
    target: &StartupCheckpointEvidence,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    worker_id: Uuid,
    controller_generation: u64,
    runtime: &RuntimeRecord,
    source_boot_id: &str,
    target_boot_id: &str,
) -> Result<StartupCheckpointEvidence> {
    super::validation::validate(target)?;
    anyhow::ensure!(
        target.terminal_id == terminal_id
            && target.runtime_epoch == runtime_epoch
            && target.worker_id == worker_id
            && target.controller_generation == controller_generation
            && target.worker.pid == runtime.worker_pid
            && target.worker.start_time == runtime.worker_start_time
            && target.child.is_some_and(|child| {
                child.pid == runtime.child_pid && child.start_time == runtime.child_start_time
            })
            && yolo_process_identity::boot_identity_relation(&target.boot_id, target_boot_id)
                == yolo_process_identity::BootIdentityRelation::Same,
        super::constants::IDENTITY_ERROR
    );
    let mut source = target.clone();
    source.boot_id = source_boot_id.to_owned();
    super::validation::validate(&source)?;
    Ok(source)
}
