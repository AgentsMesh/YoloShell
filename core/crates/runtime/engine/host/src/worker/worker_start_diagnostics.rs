pub(crate) const WORKER_START_PUBLIC_DETAIL: &str = "session worker could not be started";
pub(crate) const WORKER_START_FAILURE_LOG: &str = "session worker start failed";

pub(crate) fn worker_start_failure(terminal_id: Uuid, error: &anyhow::Error) -> ErrorBody {
    tracing::error!(
        %terminal_id,
        error = %format!("{error:#}"),
        message = WORKER_START_FAILURE_LOG,
    );
    ErrorBody::new(
        crate::protocol::error_code::WORKER_START_FAILED,
        WORKER_START_PUBLIC_DETAIL,
        true,
    )
}
