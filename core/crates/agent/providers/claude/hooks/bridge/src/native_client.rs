use crate::claude_hooks::{self, ClaudeNativeRequest, ClaudeNativeResponse, ClaudeNativeScope};
use crate::local_unix::LocalUnixStream;
use anyhow::{Context, Result, ensure};
use std::path::PathBuf;
use uuid::Uuid;

const INVALID: &str = "Claude native hook context is unavailable or invalid";
const PEER_INVALID: &str = "Claude native hook peer belongs to another user";

pub(super) struct NativeClient {
    socket: PathBuf,
    pub(super) outbox: PathBuf,
    pub(super) scope: ClaudeNativeScope,
}

impl NativeClient {
    pub(super) fn new(socket: PathBuf, outbox: PathBuf, scope: ClaudeNativeScope) -> Result<Self> {
        ensure!(
            socket.is_absolute() && outbox.is_absolute() && scope.is_valid(),
            INVALID
        );
        Ok(Self {
            socket,
            outbox,
            scope,
        })
    }

    pub(super) fn from_environment() -> Result<Option<Self>> {
        let Some(socket) = std::env::var_os(claude_hooks::CLAUDE_HOOK_SOCKET_ENV) else {
            return Ok(None);
        };
        let socket = PathBuf::from(socket);
        let outbox = std::env::var_os(claude_hooks::CLAUDE_HOOK_OUTBOX_ENV)
            .map(PathBuf::from)
            .context(INVALID)?;
        let scope = ClaudeNativeScope {
            terminal_id: required_uuid(yolo_agent_core::constants::YOLO_TERMINAL_ID_ENV)?,
            runtime_epoch: required_uuid(yolo_agent_core::constants::YOLO_RUNTIME_EPOCH_ENV)?,
            agent: super::native_parent::candidate()?,
        };
        Ok(Some(Self::new(socket, outbox, scope)?))
    }

    pub(super) async fn send(&self, request: &ClaudeNativeRequest) -> Result<ClaudeNativeResponse> {
        ensure!(request.is_valid() && request.scope == self.scope, INVALID);
        let mut stream = LocalUnixStream::connect(&self.socket).await?;
        let peer = stream.peer_cred()?;
        // SAFETY: geteuid has no preconditions.
        ensure!(peer.uid() == unsafe { libc::geteuid() }, PEER_INVALID);
        claude_hooks::write_frame(&mut stream, request).await?;
        Ok(claude_hooks::read_frame(&mut stream).await?)
    }
}

fn required_uuid(name: &str) -> Result<Uuid> {
    Uuid::parse_str(&std::env::var(name).context(INVALID)?).context(INVALID)
}
