use crate::claude_hooks::{
    self, ClaudeHookState, ClaudeLifecycleEvent, ClaudeNativeRequest, ClaudeNativeResponse,
    ClaudeNativeScope, ClaudeProcessIdentity, ClaudeSessionStartSource,
};
use crate::native_client::NativeClient;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;
use uuid::Uuid;

pub const DELIVERY_BUDGET: Duration = Duration::from_secs(2);
pub const SHORT_BUDGET: Duration = Duration::from_millis(100);
pub const COMPLETION_BUDGET: Duration = Duration::from_secs(3);
pub const OBSERVATION_WINDOW: Duration = Duration::from_millis(20);
pub const REVISION: u64 = 1;
pub const OTHER_SEQUENCE: u64 = 91;
pub const WRONG_SCHEMA: u32 = u32::MAX;
pub const WRONG_CODE: &str = "unexpected-response";
const PRIVATE_DIRECTORY_MODE: u32 = 0o700;
const SOCKET: &str = "native.sock";
const TRANSCRIPT: &str = "transcript.jsonl";
const AGENT_PID: u32 = 10_123;
const AGENT_START: u64 = 10_124;

pub struct Fixture {
    pub home: tempfile::TempDir,
    pub client: NativeClient,
    pub session: String,
}

impl Fixture {
    pub fn new() -> Self {
        let home = tempfile::tempdir().unwrap();
        std::fs::set_permissions(
            home.path(),
            std::fs::Permissions::from_mode(PRIVATE_DIRECTORY_MODE),
        )
        .unwrap();
        let scope = ClaudeNativeScope {
            terminal_id: Uuid::new_v4(),
            runtime_epoch: Uuid::new_v4(),
            agent: ClaudeProcessIdentity {
                pid: AGENT_PID,
                start_time: AGENT_START,
            },
        };
        let client = NativeClient::new(
            home.path().join(SOCKET),
            home.path().join(claude_hooks::CLAUDE_NATIVE_OUTBOX_FILE),
            scope,
        )
        .unwrap();
        Self {
            home,
            client,
            session: Uuid::new_v4().to_string(),
        }
    }

    pub fn listener(&self) -> tokio::net::UnixListener {
        tokio::net::UnixListener::bind(self.home.path().join(SOCKET)).unwrap()
    }

    pub fn start(&self) -> ClaudeLifecycleEvent {
        ClaudeLifecycleEvent::SessionStart {
            event_id: Uuid::new_v4(),
            provider_session_id: self.session.clone(),
            source: ClaudeSessionStartSource::Fresh,
            cwd: self.home.path().to_path_buf(),
            transcript_path: self.home.path().join(TRANSCRIPT),
            state: ClaudeHookState::WaitingForInput,
        }
    }

    pub fn running(&self) -> ClaudeLifecycleEvent {
        ClaudeLifecycleEvent::StateChanged {
            event_id: Uuid::new_v4(),
            provider_session_id: self.session.clone(),
            state: ClaudeHookState::Running,
        }
    }

    pub fn next(&self) -> Option<ClaudeNativeRequest> {
        claude_hooks::native_outbox::next(&self.client.outbox, self.client.scope).unwrap()
    }
}

pub fn accepted(request: &ClaudeNativeRequest) -> ClaudeNativeResponse {
    ClaudeNativeResponse {
        schema_version: claude_hooks::CLAUDE_NATIVE_SCHEMA_VERSION,
        accepted: true,
        code: claude_hooks::CLAUDE_HOOK_ACCEPTED.to_owned(),
        scope: request.scope,
        event_sequence: request.event_sequence,
        event_id: request.event.event_id(),
        invocation_id: Some(Uuid::new_v4()),
        revision: Some(REVISION),
        interaction: None,
    }
}

pub async fn receive(
    listener: &tokio::net::UnixListener,
) -> (tokio::net::UnixStream, ClaudeNativeRequest) {
    let (mut stream, _) = tokio::time::timeout(COMPLETION_BUDGET, listener.accept())
        .await
        .unwrap()
        .unwrap();
    let request = tokio::time::timeout(COMPLETION_BUDGET, claude_hooks::read_frame(&mut stream))
        .await
        .unwrap()
        .unwrap();
    (stream, request)
}

pub async fn respond(stream: &mut tokio::net::UnixStream, response: &ClaudeNativeResponse) {
    claude_hooks::write_frame(stream, response).await.unwrap();
}
