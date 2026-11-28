use uuid::Uuid;
use yolo_application_support::constants::RUNTIME_RECORD_SCHEMA_VERSION;
use yolo_terminal_model::{BootstrapDelivery, RuntimeRecord};
use yolo_wire::ErrorBody;

pub const WORKER_BOOTSTRAP_FAILED: &str = "worker bootstrap delivered no input";
pub const WORKER_BOOTSTRAP_UNKNOWN: &str = "worker bootstrap delivery is unknown";
pub const WORKER_BOOTSTRAP_PENDING: &str = "worker bootstrap delivery is pending";

pub fn validate_runtime_bootstrap(
    delivery: BootstrapDelivery,
) -> std::result::Result<(), ErrorBody> {
    let (code, message) = match delivery {
        BootstrapDelivery::NotRequired
        | BootstrapDelivery::Delivered
        | BootstrapDelivery::Survivor => return Ok(()),
        BootstrapDelivery::Pending => (
            yolo_wire::error_code::DELIVERY_UNKNOWN,
            WORKER_BOOTSTRAP_PENDING,
        ),
        BootstrapDelivery::Unknown => (
            yolo_wire::error_code::DELIVERY_UNKNOWN,
            WORKER_BOOTSTRAP_UNKNOWN,
        ),
        BootstrapDelivery::Failed => (
            yolo_wire::error_code::WORKER_START_FAILED,
            WORKER_BOOTSTRAP_FAILED,
        ),
    };
    Err(ErrorBody::new(code, message, false))
}

/// Verify the recorded worker before using its RPC endpoint.
pub fn validate_live_worker(
    record: &RuntimeRecord,
    terminal_id: Uuid,
    boot: &str,
) -> std::result::Result<(), ErrorBody> {
    if record.schema_version != RUNTIME_RECORD_SCHEMA_VERSION {
        return Err(ErrorBody::new(
            yolo_wire::error_code::IDENTITY_MISMATCH,
            "runtime record schema cannot establish process identity",
            false,
        ));
    }
    if record.terminal_id != terminal_id {
        return Err(ErrorBody::new(
            yolo_wire::error_code::IDENTITY_MISMATCH,
            "runtime session identity mismatch",
            false,
        ));
    }
    validate_runtime_bootstrap(record.bootstrap_delivery)?;
    if yolo_process::boot_identity_relation(&record.boot_id, boot)
        == yolo_process::BootIdentityRelation::Different
    {
        return Err(ErrorBody::new(
            yolo_wire::error_code::HOST_REBOOTED,
            "runtime belongs to a previous host boot",
            false,
        ));
    }
    if record.worker_start_time == 0 || record.child_start_time == 0 {
        return Err(ErrorBody::new(
            yolo_wire::error_code::IDENTITY_MISMATCH,
            "runtime record contains an unverifiable process identity",
            false,
        ));
    }
    if !yolo_process::is_process_alive(record.worker_pid) {
        return Err(ErrorBody::new(
            yolo_wire::error_code::WORKER_UNREACHABLE,
            "session worker is not running",
            true,
        ));
    }
    let actual = yolo_process::process_start_time(record.worker_pid).map_err(|_| {
        ErrorBody::new(
            yolo_wire::error_code::WORKER_UNREACHABLE,
            "cannot validate worker process identity",
            true,
        )
    })?;
    if actual != record.worker_start_time {
        return Err(ErrorBody::new(
            yolo_wire::error_code::IDENTITY_MISMATCH,
            "worker PID was reused by another process",
            false,
        ));
    }
    Ok(())
}
