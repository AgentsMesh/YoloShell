#[path = "local_startup_support/constants.rs"]
mod constants;
use crate::ProcessIdentity;
pub use constants::*;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    time::{Instant, timeout_at},
};
use yolo_hub_client::Client;
use yolo_hub_local_startup::{BootstrapDocument, ConnectionDocument};

pub fn deadline() -> Instant {
    Instant::now() + TEST_TIMEOUT
}

pub fn hub_binary() -> PathBuf {
    crate::bazel_runfile(&std::env::var(HUB_BINARY_ENV).unwrap()).unwrap()
}

pub async fn retain_fixture_host(harness: &crate::Harness) -> yolo_client::HostHold {
    let socket = harness.paths().host_socket().unwrap();
    let client = yolo_client::Client::managed_local(harness.home.path(), &socket).unwrap();
    let hold = client.retain_host().await.unwrap_or_else(|error| {
        panic!(
            "{RETAIN_HOST_FAILURE}: {error:?}; host={:?}; alive={}; stderr={}",
            harness.host_identity(),
            harness.host_identity().matches(),
            harness.host_stderr_tail()
        )
    });
    assert_eq!(hold.initial_host_pid(), harness.host_identity().pid);
    assert!(harness.host_identity().matches());
    hold
}

pub async fn bootstrap(home: &Path, state: &Path) -> ConnectionDocument {
    let executable = crate::bazel_runfile(&std::env::var(HUB_BINARY_ENV).unwrap()).unwrap();
    let mut child = tokio::process::Command::new(executable)
        .args(yolo_launch_contract::ProcessRole::Hub.arguments())
        .arg(BOOTSTRAP)
        .arg(STATE)
        .arg(state)
        .arg(HOST_HOME)
        .arg(home)
        .arg(HOST_EXECUTABLE)
        .arg(crate::runtime_binary())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .unwrap();
    let output = child.stdout.take().unwrap();
    let error = child.stderr.take().unwrap();
    let result = timeout_at(deadline(), async {
        tokio::try_join!(child.wait(), capture(output), capture(error))
    })
    .await;
    match result {
        Ok(Ok((status, output, error))) => {
            assert!(
                status.success(),
                "Hub bootstrap failed: {}",
                String::from_utf8_lossy(&error)
            );
            let document: BootstrapDocument = yolo_capnp_codec::decode(&output).unwrap();
            assert_eq!(document.state_path, state.canonicalize().unwrap());
            assert_eq!(document.host_home, home.canonicalize().unwrap());
            document.connection
        }
        result => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            panic!("Hub bootstrap did not settle: {result:?}");
        }
    }
}

async fn capture(reader: impl AsyncRead + Unpin) -> std::io::Result<Vec<u8>> {
    let mut result = Vec::new();
    reader
        .take(MAX_CAPTURE_BYTES)
        .read_to_end(&mut result)
        .await?;
    Ok(result)
}

pub struct Services {
    pub host: ProcessIdentity,
    pub hub: ProcessIdentity,
}
impl Services {
    pub async fn capture(home: &Path, document: &ConnectionDocument) -> Self {
        let (host, hub) = Self::inspect(home, document).await;
        Self { host, hub }
    }
    pub async fn inspect(
        home: &Path,
        document: &ConnectionDocument,
    ) -> (ProcessIdentity, ProcessIdentity) {
        let socket = yolo_runtime::paths::Paths::new(home).host_socket().unwrap();
        let client = yolo_client::Client::managed_local(home, &socket).unwrap();
        let status = client.status().await.unwrap();
        let host = ProcessIdentity::capture(status.host_pid).unwrap();
        let fresh = client.status().await.unwrap();
        assert_eq!(fresh.host_pid, host.pid);
        assert!(host.matches());
        let client = Client::connect_local(&document.socket_path, document.entry_hub, deadline())
            .await
            .unwrap();
        let stream = client.into_channel().unwrap().into_stream().unwrap();
        let pid = u32::try_from(stream.get_ref().peer_cred().unwrap().pid().unwrap()).unwrap();
        let hub = ProcessIdentity::capture(pid).unwrap();
        // A fresh exchange after identity capture proves that this connection still
        // belongs to the process being retained, rather than a reused PID.
        Client::open(stream, document.entry_hub, deadline())
            .await
            .unwrap();
        assert!(hub.matches());
        (host, hub)
    }
    pub async fn stop(self) {
        self.hub.signal(libc::SIGTERM).unwrap();
        crate::wait_for_identity_gone(self.hub).await;
        self.host.signal(libc::SIGINT).unwrap();
        crate::wait_for_identity_gone(self.host).await;
    }
}
impl Drop for Services {
    fn drop(&mut self) {
        let _ = self.hub.signal(libc::SIGTERM);
        let _ = self.host.signal(libc::SIGINT);
    }
}
