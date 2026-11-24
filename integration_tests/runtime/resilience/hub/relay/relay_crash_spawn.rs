use super::*;
use crate::hub::{files_runtime, local_startup_support::hub_binary};
use std::process::{Command, Stdio};
use yolo_hub_local_startup::{
    FILE_HELPER_ARGUMENT, HOST_SOCKET_ARGUMENT, PEER_CONFIG_ARGUMENT,
};
use yolo_launch_contract::{HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT};

impl RelayProcess {
    pub(super) fn spawn_configured(
        harness: &crate::Harness,
        state: &Path,
        socket: &Path,
        peer: PeerIdentity,
        configuration: Option<&Path>,
    ) -> Self {
        let log = tempfile::NamedTempFile::new_in(state).unwrap();
        let mut command = Command::new(hub_binary());
        command
            .args(yolo_launch_contract::ProcessRole::Hub.arguments())
            .arg(HUB_STATE_ARGUMENT)
            .arg(state)
            .arg(HOST_HOME_ARGUMENT)
            .arg(harness.home.path())
            .arg(HOST_SOCKET_ARGUMENT)
            .arg(harness.paths().host_socket().unwrap())
            .arg(FILE_HELPER_ARGUMENT)
            .arg(files_runtime::helper())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log.reopen().unwrap());
        if let Some(configuration) = configuration {
            command.arg(PEER_CONFIG_ARGUMENT).arg(configuration);
        }
        let child = ScopedWaitableChild::spawn_detached_before_deadline(
            &mut command,
            deadline().into_std(),
        )
        .unwrap();
        let identity = crate::ProcessIdentity::capture(child.process_id()).unwrap();
        Self {
            child,
            identity,
            peer,
            socket: socket.to_path_buf(),
            _log: log,
        }
    }
}
