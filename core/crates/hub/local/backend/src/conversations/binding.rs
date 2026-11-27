use yolo_agent_control_contract::NativeControlObservation;
use yolo_conversation_protocol::ConversationError;

pub(crate) fn recheck(
    before: &NativeControlObservation,
    after: Result<NativeControlObservation, ConversationError>,
) -> Result<(), ConversationError> {
    let after = after?;
    if same_native_run(before, &after) { Ok(()) } else { Err(ConversationError::BindingChanged) }
}

/// Every observation is authenticated independently. Content reads do not require
/// unchanged hook activity, readiness or timestamps; they do require the same
/// origin execution, Worker, native process, conversation and evidence association.
pub(crate) fn same_native_run(
    before: &NativeControlObservation,
    after: &NativeControlObservation,
) -> bool {
    let left = before.binding();
    let right = after.binding();
    let left_worker = before.witness().fields();
    let right_worker = after.witness().fields();
    left.terminal_id == right.terminal_id
        && left.kind == right.kind
        && left.provider_session_id == right.provider_session_id
        && left.executable == right.executable
        && left.cwd == right.cwd
        && left.evidence_path == right.evidence_path
        && left.data_home == right.data_home
        && left.process_pid == right.process_pid
        && left.process_start_time == right.process_start_time
        && left_worker.terminal_id == right_worker.terminal_id
        && left_worker.runtime_epoch == right_worker.runtime_epoch
        && left_worker.worker_id == right_worker.worker_id
        && left_worker.boot_id == right_worker.boot_id
}
