use super::*;
use crate::claude_hooks::{ClaudeHookState, ClaudeSessionEndReason, ClaudeSessionStartSource};

pub(super) fn session_start(root: &Path, transcript: &Path, session: &str) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::SessionStart {
        event_id: Uuid::new_v4(),
        provider_session_id: session.to_owned(),
        source: ClaudeSessionStartSource::Fresh,
        cwd: root.to_path_buf(),
        transcript_path: transcript.to_path_buf(),
        state: ClaudeHookState::Running,
    }
}

pub(super) fn cwd_changed(root: &Path, transcript: &Path, session: &str) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::CwdChanged {
        event_id: Uuid::new_v4(),
        provider_session_id: session.to_owned(),
        cwd: root.join(NESTED_DIRECTORY),
        transcript_path: transcript.to_path_buf(),
    }
}

pub(super) fn state_changed(session: &str) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::StateChanged {
        event_id: Uuid::new_v4(),
        provider_session_id: session.to_owned(),
        state: ClaudeHookState::WaitingForInput,
    }
}

pub(super) fn session_end(session: &str) -> ClaudeLifecycleEvent {
    ClaudeLifecycleEvent::SessionEnd {
        event_id: Uuid::new_v4(),
        provider_session_id: session.to_owned(),
        reason: ClaudeSessionEndReason::Resume,
    }
}

pub(super) fn credentials() -> ClaudeHookCredentials {
    ClaudeHookCredentials {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
    }
}

pub(super) fn before() -> crate::sha256_digest::Sha256Digest {
    crate::sha256_digest::Sha256Digest::repeated(BEFORE_DIGEST_BYTE)
}

pub(super) fn after() -> crate::sha256_digest::Sha256Digest {
    crate::sha256_digest::Sha256Digest::repeated(AFTER_DIGEST_BYTE)
}
