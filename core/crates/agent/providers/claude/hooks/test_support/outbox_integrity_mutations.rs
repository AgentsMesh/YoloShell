use super::*;
use crate::claude_hooks::ClaudeHookOperation;

pub(super) fn sequence(request: &mut ClaudeHookRequest) -> &mut u64 {
    let ClaudeHookOperation::Lifecycle { event_sequence, .. } = &mut request.operation else {
        unreachable!()
    };
    event_sequence
}

pub(super) fn credentials(state: &mut OutboxState) -> &mut ClaudeHookCredentials {
    &mut state.invocations[FIRST_QUEUE].pending[STATE_CHANGED_INDEX]
        .request
        .credentials
}
