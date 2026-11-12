use super::handler;
use uuid::Uuid;

#[test]
fn rejection_response_never_exposes_internal_error_detail() {
    use crate::claude_hooks::*;
    const AGENT_PID: u32 = 56_001;
    const AGENT_START: u64 = 56_002;
    let request = ClaudeNativeRequest {
        schema_version: CLAUDE_NATIVE_SCHEMA_VERSION,
        scope: ClaudeNativeScope {
            terminal_id: Uuid::new_v4(),
            runtime_epoch: Uuid::new_v4(),
            agent: ClaudeProcessIdentity {
                pid: AGENT_PID,
                start_time: AGENT_START,
            },
        },
        event_sequence: INITIAL_CLAUDE_EVENT_SEQUENCE,
        event: ClaudeLifecycleEvent::SessionEnd {
            event_id: Uuid::new_v4(),
            provider_session_id: Uuid::new_v4().to_string(),
            reason: ClaudeSessionEndReason::Other,
        },
    };
    let response = handler::rejection(&request);
    assert!(!response.accepted);
    assert_eq!(response.code, crate::claude_hooks::CLAUDE_HOOK_REJECTED);
    assert_eq!(response.revision, None);
    assert_eq!(response.invocation_id, None);
    assert_eq!(response.event_id, request.event.event_id());
}
