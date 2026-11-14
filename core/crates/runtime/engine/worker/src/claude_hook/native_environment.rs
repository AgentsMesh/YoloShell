use crate::claude_hooks::*;
use crate::worker::WorkerNamespace;
use anyhow::{Result, ensure};
use portable_pty::CommandBuilder;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const INVALID: &str = "Claude native hook runtime context is invalid";

#[derive(Clone, Debug)]
pub(in crate::worker) struct ClaudeHookEnvironment {
    socket: PathBuf,
    outbox: PathBuf,
    executable: Arc<super::native_executable::HookExecutable>,
}

impl ClaudeHookEnvironment {
    pub(super) fn new(
        namespace: &WorkerNamespace,
        paths: &crate::paths::Paths,
        terminal_id: uuid::Uuid,
        executable: PathBuf,
    ) -> Result<Self> {
        let socket = namespace.claude_hook_socket(terminal_id)?;
        let directory = paths
            .session_dir(terminal_id)
            .join(CLAUDE_HOOK_OUTBOX_DIRECTORY);
        namespace
            .lease()
            .ensure_directory_with_commit_fence(&directory, || namespace.lease().verify())?;
        let executable = Arc::new(super::native_executable::HookExecutable::open(executable)?);
        ensure!(socket.is_absolute(), INVALID);
        Ok(Self {
            socket,
            outbox: directory.join(CLAUDE_NATIVE_OUTBOX_FILE),
            executable,
        })
    }

    pub(in crate::worker) fn configure(&self, command: &mut CommandBuilder) -> Result<()> {
        self.executable.verify()?;
        command.env(CLAUDE_HOOK_SOCKET_ENV, &self.socket);
        command.env(CLAUDE_HOOK_OUTBOX_ENV, &self.outbox);
        command.env(NATIVE_HOOK_EXECUTABLE_ENV, self.executable.path());
        Ok(())
    }

    pub(in crate::worker) fn socket_path(&self) -> &Path {
        &self.socket
    }
    pub(super) fn outbox_path(&self) -> &Path {
        &self.outbox
    }
    pub(super) fn executable(&self) -> &super::native_executable::HookExecutable {
        &self.executable
    }
}
