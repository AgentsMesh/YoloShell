use super::*;

impl SocketNamespaceLocation {
    pub(super) fn hub_socket(&self) -> Result<PathBuf> {
        checked(self.directory.join(HUB_SOCKET_FILE_NAME))
    }

    pub(super) fn connector_socket(&self) -> Result<PathBuf> {
        checked(self.directory.join(CONNECTOR_SOCKET_FILE_NAME))
    }

    pub(super) fn host_socket(&self) -> Result<PathBuf> {
        checked(self.directory.join(HOST_SOCKET_FILE_NAME))
    }

    pub(super) fn worker_socket(&self, id: Uuid) -> Result<PathBuf> {
        checked(self.directory.join(format!("{id}{SOCKET_FILE_EXTENSION}")))
    }

    pub(super) fn custody_guardian_socket(&self, id: Uuid) -> Result<PathBuf> {
        checked(self.directory.join(format!(
            "{CUSTODY_GUARDIAN_SOCKET_PREFIX}{}{SOCKET_FILE_EXTENSION}",
            id.simple()
        )))
    }
}

impl SocketNamespace {
    pub fn hub_socket(&self) -> Result<PathBuf> {
        self.location.hub_socket()
    }

    pub fn connector_socket(&self) -> Result<PathBuf> {
        self.location.connector_socket()
    }

    pub fn host_socket(&self) -> Result<PathBuf> {
        self.location.host_socket()
    }

    pub fn worker_socket(&self, id: Uuid) -> Result<PathBuf> {
        self.location.worker_socket(id)
    }

    pub fn claude_hook_socket(&self, id: Uuid) -> Result<PathBuf> {
        checked_claude_hook_path(&self.location.directory, id)
    }

    pub fn codex_hook_socket(&self, id: Uuid) -> Result<PathBuf> {
        checked_codex_hook_path(&self.location.directory, id)
    }

    pub fn qwen_hook_socket(&self, id: Uuid) -> Result<PathBuf> {
        checked_qwen_hook_path(&self.location.directory, id)
    }

    pub fn provider_hook_socket(&self, id: Uuid) -> Result<PathBuf> {
        checked_provider_hook_path(&self.location.directory, id)
    }

    pub fn custody_guardian_socket(&self, id: Uuid) -> Result<PathBuf> {
        self.location.custody_guardian_socket(id)
    }
}

fn checked_hook_path(directory: &std::path::Path, prefix: &str, id: Uuid) -> Result<PathBuf> {
    checked(directory.join(format!("{prefix}{}{SOCKET_FILE_EXTENSION}", id.simple())))
}

pub(super) fn checked_claude_hook_path(directory: &std::path::Path, id: Uuid) -> Result<PathBuf> {
    checked_hook_path(directory, CLAUDE_HOOK_SOCKET_PREFIX, id)
}

pub(super) fn checked_codex_hook_path(directory: &std::path::Path, id: Uuid) -> Result<PathBuf> {
    checked_hook_path(directory, CODEX_HOOK_SOCKET_PREFIX, id)
}

pub(super) fn checked_qwen_hook_path(directory: &std::path::Path, id: Uuid) -> Result<PathBuf> {
    checked_hook_path(directory, QWEN_HOOK_SOCKET_PREFIX, id)
}

pub(super) fn checked_provider_hook_path(
    directory: &std::path::Path,
    id: Uuid,
) -> Result<PathBuf> {
    checked_hook_path(directory, PROVIDER_HOOK_SOCKET_PREFIX, id)
}

fn checked(path: PathBuf) -> Result<PathBuf> {
    socket_path::validate(&path)?;
    Ok(path)
}
