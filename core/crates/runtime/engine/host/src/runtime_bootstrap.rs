use super::*;

pub(super) fn validate_runtime_bootstrap(
    delivery: BootstrapDelivery,
) -> std::result::Result<(), ErrorBody> {
    let (code, message) = match delivery {
        BootstrapDelivery::NotRequired
        | BootstrapDelivery::Delivered
        | BootstrapDelivery::Survivor => return Ok(()),
        BootstrapDelivery::Pending => (
            crate::protocol::error_code::DELIVERY_UNKNOWN,
            WORKER_BOOTSTRAP_PENDING,
        ),
        BootstrapDelivery::Unknown => (
            crate::protocol::error_code::DELIVERY_UNKNOWN,
            WORKER_BOOTSTRAP_UNKNOWN,
        ),
        BootstrapDelivery::Failed => (
            crate::protocol::error_code::WORKER_START_FAILED,
            WORKER_BOOTSTRAP_FAILED,
        ),
    };
    Err(ErrorBody::new(code, message, false))
}

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
