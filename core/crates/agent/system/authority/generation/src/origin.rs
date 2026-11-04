use super::GenerationRequest;
use super::record::HookGenerationRecord;
use yolo_hooks_runtime::{EventKind, SessionStartSource};

/// A resumed runtime continues an existing generation: the durable binding
/// origin must survive the runtime restart so the Host resume transaction can
/// prove it adopted the same agent rather than a lookalike.
pub(super) fn inherit_session_origin(
    record: &mut HookGenerationRecord,
    request: &GenerationRequest,
    previous: Option<&HookGenerationRecord>,
) {
    if !matches!(
        request.event,
        EventKind::SessionStart {
            source: SessionStartSource::Resume,
        }
    ) {
        return;
    }
    let Some(previous) = previous else {
        return;
    };
    if previous.revoked
        || previous.scope.terminal_id != record.scope.terminal_id
        || previous.binding.kind != record.binding.kind
        || previous.binding.provider_session_id != record.binding.provider_session_id
        || previous.binding.cwd != record.binding.cwd
        || previous.binding.data_home != record.binding.data_home
        || previous.binding.evidence_path != record.binding.evidence_path
        || previous.transcript_identity != record.transcript_identity
        || previous.cwd_identity != record.cwd_identity
        || previous.data_home_identity != record.data_home_identity
    {
        return;
    }
    record.binding.detected_at_ms = previous
        .binding
        .detected_at_ms
        .min(record.binding.detected_at_ms);
}
