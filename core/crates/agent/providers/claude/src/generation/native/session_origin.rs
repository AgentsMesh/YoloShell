use super::ClaudeNativeGeneration;
use crate::claude_hooks::{ClaudeLifecycleEvent, ClaudeNativeRequest, ClaudeSessionStartSource};

pub(super) fn inherit(
    next: &mut ClaudeNativeGeneration,
    request: &ClaudeNativeRequest,
    previous: &[ClaudeNativeGeneration],
) {
    if !matches!(
        request.event,
        ClaudeLifecycleEvent::SessionStart {
            source: ClaudeSessionStartSource::Resume,
            ..
        }
    ) {
        return;
    }
    let origin = previous
        .iter()
        .filter(|prior| {
            !prior.revoked
                && prior.scope.terminal_id == next.scope.terminal_id
                && prior.binding.provider_session_id == next.binding.provider_session_id
                && prior.binding.cwd == next.binding.cwd
                && prior.binding.data_home == next.binding.data_home
                && prior.binding.evidence_path == next.binding.evidence_path
                && prior.transcript_identity == next.transcript_identity
                && prior.cwd_identity == next.cwd_identity
                && prior.data_home_identity == next.data_home_identity
        })
        .map(|prior| prior.binding.detected_at_ms)
        .min();
    if let Some(origin) = origin {
        next.binding.detected_at_ms = origin;
    }
}
