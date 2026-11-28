use super::*;

#[derive(Debug)]
pub(super) enum BootstrapWorkerAction {
    AwaitFinal,
    Authenticate,
    Uncertain(anyhow::Error),
}

pub(super) fn bootstrap_worker_start(delivery: BootstrapDelivery) -> Result<BootstrapWorkerAction> {
    match delivery {
        BootstrapDelivery::NotRequired
        | BootstrapDelivery::Delivered
        | BootstrapDelivery::Survivor => {
            Ok(BootstrapWorkerAction::Authenticate)
        }
        BootstrapDelivery::Pending => Ok(BootstrapWorkerAction::AwaitFinal),
        BootstrapDelivery::Failed => bail!(WORKER_BOOTSTRAP_FAILED),
        BootstrapDelivery::Unknown => Ok(BootstrapWorkerAction::Uncertain(anyhow::anyhow!(
            WORKER_BOOTSTRAP_UNKNOWN,
        ))),
    }
}

pub(super) fn pending_bootstrap_identities_live(record: &RuntimeRecord) -> bool {
    record.bootstrap_delivery == BootstrapDelivery::Pending
        && !process_identity_is_gone(record.worker_pid, record.worker_start_time)
        && !process_identity_is_gone(record.child_pid, record.child_start_time)
}


pub(super) fn dead_pending_record(terminal_id: Uuid, record: &RuntimeRecord) -> bool {
    record.terminal_id == terminal_id && record.bootstrap_delivery == BootstrapDelivery::Pending
        && !pending_bootstrap_identities_live(record)
}
