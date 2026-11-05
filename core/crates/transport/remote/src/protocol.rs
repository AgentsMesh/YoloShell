use super::RemoteError;
use super::constants::{
    BRIDGE_COMMAND, HOME_ARGUMENT, MAX_REMOTE_PATH_BYTES, NUL_CHARACTER,
    REQUIRED_RUNTIME_CAPABILITIES, RUNTIME_SERVER_NAME, STDIO_ARGUMENT,
};
use super::file::scoped_absolute;
use crate::model::{HostStatus, PROTOCOL_VERSION};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeRole {
    Control,
    Attachment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRuntimeSpec {
    executable: String,
    home: String,
}

impl RemoteRuntimeSpec {
    pub fn new(
        executable: impl Into<PathBuf>,
        home: impl Into<PathBuf>,
    ) -> Result<Self, RemoteError> {
        let executable = validated_remote_path(executable.into())?;
        let home = validated_remote_path(home.into())?;
        Ok(Self { executable, home })
    }

    pub fn executable(&self) -> &Path {
        Path::new(&self.executable)
    }

    pub fn home(&self) -> &Path {
        Path::new(&self.home)
    }

    pub fn args(&self) -> Vec<String> {
        yolo_launch_contract::ProcessRole::Host.arguments().into_iter().map(str::to_owned).chain([
            BRIDGE_COMMAND.into(),
            STDIO_ARGUMENT.into(),
            HOME_ARGUMENT.into(),
            self.home.clone(),
        ]).collect()
    }

    pub(crate) fn executable_text(&self) -> &str {
        &self.executable
    }
}

fn validated_remote_path(path: PathBuf) -> Result<String, RemoteError> {
    let text = path.to_string_lossy();
    if !scoped_absolute(&path) || text.len() > MAX_REMOTE_PATH_BYTES || text.contains(NUL_CHARACTER)
    {
        return Err(RemoteError::InvalidRuntime(
            "remote path must be bounded, UTF-8, canonical, scoped, and absolute".into(),
        ));
    }
    Ok(text.into_owned())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeChannelRequest {
    pub role: RuntimeRole,
    pub runtime: RemoteRuntimeSpec,
}

impl RuntimeChannelRequest {
    pub fn control(runtime: RemoteRuntimeSpec) -> Self {
        Self {
            role: RuntimeRole::Control,
            runtime,
        }
    }

    pub fn attachment(runtime: RemoteRuntimeSpec) -> Self {
        Self {
            role: RuntimeRole::Attachment,
            runtime,
        }
    }

    pub fn role(&self) -> RuntimeRole {
        self.role
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeIdentity {
    pub connection_id: Uuid,
    pub boot_id: String,
    pub controller_generation: u64,
}

impl RuntimeIdentity {
    pub(crate) fn from_status(connection_id: Uuid, status: &HostStatus) -> Self {
        Self {
            connection_id,
            boot_id: status.boot_id.clone(),
            controller_generation: status.controller_generation,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteState {
    Ready(RuntimeIdentity),
    Disconnected { previous_connection_id: Uuid },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconnectReport {
    pub previous: RuntimeIdentity,
    pub current: RuntimeIdentity,
    pub same_boot: bool,
}

pub(crate) fn validate_runtime(
    server_name: &str,
    hello_version: u16,
    status: &HostStatus,
    spec: &RemoteRuntimeSpec,
) -> Result<(), RemoteError> {
    if server_name != RUNTIME_SERVER_NAME
        || hello_version != PROTOCOL_VERSION
        || status.protocol_version != PROTOCOL_VERSION
        || status.home != spec.home
    {
        return Err(RemoteError::RuntimeIncompatible(
            "server identity, protocol version, or runtime home differs".into(),
        ));
    }
    let missing = REQUIRED_RUNTIME_CAPABILITIES.iter().find(|capability| {
        !status
            .capabilities
            .iter()
            .any(|actual| actual == **capability)
    });
    if let Some(capability) = missing {
        return Err(RemoteError::RuntimeIncompatible(format!(
            "missing capability {capability}"
        )));
    }
    Ok(())
}
