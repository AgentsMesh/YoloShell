use super::constants::{HOST_READY_POLL_INTERVAL, HOST_READY_TIMEOUT};
use super::status::RelayStatusTarget;
use super::{RelayError, RelayStatus};
use crate::client::{Client, ClientError};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[path = "host/spawn.rs"]
mod spawn;
use spawn::spawn_host;
#[path = "host/broker.rs"]
mod broker;
#[path = "host/preparation.rs"]
mod preparation;
#[path = "host/probe.rs"]
mod probe;
#[path = "host/recovery.rs"]
mod recovery;
pub(crate) use preparation::PreparedSession;
#[cfg(all(test, feature = "yolo-unit-tests"))]
use preparation::{SessionPlan, session_plan};
use probe::poll_host_status;

#[derive(Clone, Debug)]
pub(super) struct HostLaunch {
    home: PathBuf,
    socket: PathBuf,
    executable: PathBuf,
}

#[derive(Clone, Debug)]
pub struct RelayRuntime {
    client: Client,
    launch: Option<HostLaunch>,
    wait_for_broker: bool,
}

impl RelayRuntime {
    pub fn existing(client: Client) -> Self {
        Self {
            client,
            launch: None,
            wait_for_broker: false,
        }
    }

    pub fn managed(
        home: impl Into<PathBuf>,
        socket: impl Into<PathBuf>,
        host_executable: Option<PathBuf>,
    ) -> Result<Self, RelayError> {
        let executable = match host_executable {
            Some(path) => path,
            None => runtime_executable_path(&std::env::current_exe()?)?,
        };
        let launch = HostLaunch {
            home: home.into(),
            socket: socket.into(),
            executable,
        };
        Ok(Self {
            client: Client::managed_local(&launch.home, &launch.socket)?,
            launch: Some(launch),
            wait_for_broker: false,
        })
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub(crate) async fn prepare(
        &self,
        terminal_id: Uuid,
        status: &dyn RelayStatusTarget,
    ) -> Result<PreparedSession, RelayError> {
        if let Some(executable) = self.ensure_host().await? {
            status.send_status(RelayStatus::HostStarted { executable });
        }
        let prepared = self.ensure_session_attachable(terminal_id).await?;
        if prepared == PreparedSession::UserTerminated {
            status.send_status(RelayStatus::SessionEnding);
            return Err(RelayError::SessionNotAttachable(terminal_id));
        }
        if prepared.was_resumed() {
            status.send_status(RelayStatus::SessionResumed);
        }
        Ok(prepared)
    }

    async fn ensure_host(&self) -> Result<Option<PathBuf>, RelayError> {
        if self.wait_for_broker {
            return self.wait_for_broker().await;
        }
        match self.client.status().await {
            Ok(_) => return Ok(None),
            Err(error) if self.launch.is_none() || !observation_interrupted(&error) => {
                return Err(observation_error(error));
            }
            Err(_) => {}
        }
        let launch = self
            .launch
            .as_ref()
            .expect("managed runtime has launch data");
        spawn_host(launch)?;
        let deadline = tokio::time::Instant::now() + HOST_READY_TIMEOUT;
        while tokio::time::Instant::now() < deadline {
            let status = match tokio::time::timeout_at(deadline, self.client.status()).await {
                Ok(status) => status,
                Err(_) => break,
            };
            match poll_host_status(status)? {
                true => return Ok(Some(launch.executable.clone())),
                false => {
                    let next = tokio::time::Instant::now() + HOST_READY_POLL_INTERVAL;
                    tokio::time::sleep_until(next.min(deadline)).await;
                }
            }
        }
        Err(RelayError::HostStartupTimeout {
            socket: launch.socket.clone(),
        })
    }
}

impl From<Client> for RelayRuntime {
    fn from(client: Client) -> Self {
        Self::existing(client)
    }
}

pub fn runtime_executable_path(executable: &Path) -> Result<PathBuf, RelayError> {
    if !executable.is_absolute() || executable.file_name().is_none() {
        return Err(RelayError::InvalidRuntimeExecutable(
            executable.to_path_buf(),
        ));
    }
    Ok(executable.to_path_buf())
}

pub(crate) fn is_transport_error(error: &ClientError) -> bool {
    matches!(
        error,
        ClientError::Deadline
            | ClientError::Protocol(
                crate::protocol::ProtocolError::Io(_) | crate::protocol::ProtocolError::EndOfStream
            )
    )
}

pub(crate) fn observation_interrupted(error: &ClientError) -> bool {
    is_transport_error(error)
        || matches!(
            error,
            ClientError::Protocol(crate::protocol::ProtocolError::OutcomeUnknown)
        )
}

pub(crate) fn observation_error(error: ClientError) -> RelayError {
    if !is_transport_error(&error) && observation_interrupted(&error) {
        RelayError::ObservationInterrupted(error)
    } else {
        error.into()
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/host/branch_tests.rs"]
mod branch_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/host/completed_tests.rs"]
mod completed_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../integration_tests/host_deadline_tests.rs"]
mod deadline_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/host_runtime_tests.rs"]
mod runtime_tests;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[path = "../unit_tests/host_tests.rs"]
mod tests;
