use super::HostLaunch;
use crate::cli::constants::DETACHED_SESSION_ARGUMENT;
use crate::process::{DetachedStandardError, spawn_self_detaching};
use crate::relay::RelayError;
use crate::relay::constants::{HOME_ARGUMENT, HOST_SERVE_COMMAND, SOCKET_ARGUMENT};

const HOST_REAPER_THREAD: &str = "yolo-host-reaper";

pub(super) fn spawn_host(launch: &HostLaunch) -> Result<(), RelayError> {
    let args = crate::cli::ProcessRole::Host.prepend([
        HOST_SERVE_COMMAND.into(),
        DETACHED_SESSION_ARGUMENT.into(),
        HOME_ARGUMENT.into(),
        launch.home.as_os_str().to_owned(),
        SOCKET_ARGUMENT.into(),
        launch.socket.as_os_str().to_owned(),
    ]);
    let mut child = spawn_self_detaching(&launch.executable, &args, DetachedStandardError::Null)
        .map_err(|source| RelayError::HostSpawn {
            executable: launch.executable.clone(),
            source,
        })?;
    std::thread::Builder::new()
        .name(HOST_REAPER_THREAD.to_owned())
        .spawn(move || {
            let _ = child.wait();
        })?;
    Ok(())
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/host/spawn_tests.rs"]
mod tests;
