use super::super::*;
use crate::remote::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

pub use super::support_channel::OpenStep;
pub use super::support_connection::FakeConnection;

pub const HOST_ID: &str = "build-host";
pub const ALLOWED_PROGRAM: &str = "/usr/bin/git";
pub const READ_ROOT: &str = "/srv/project";
pub const READ_PATH: &str = "/srv/project/file";
pub const CONFIG_REVISION: &str = "test-config-revision";
pub const PROFILE_AUTHORITY_SHA256: &str =
    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
pub const PLATFORM_OUTPUT: &[u8] = b"Darwin";
pub const ARCHITECTURE_OUTPUT: &[u8] = b"arm64";
pub const VERSION_OUTPUT: &[u8] = b"yolo 1.0.0";
pub const RUNTIME_HOME: &str = "/var/lib/yolo";
pub const PROXY_REQUEST_ID: &str = "proxy-request";
pub const SATURATED_MANAGEMENT_CLIENT_LIMIT: usize = 1;
pub const SATURATED_RUNTIME_CLIENT_LIMIT: usize = 1;

#[derive(Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ProxyReply {
    pub connection_id: Uuid,
    pub channel_id: u64,
}

#[derive(Clone, Copy)]
pub enum ConnectStep {
    Connection(usize),
    TransportFailure,
    AuthenticationFailure,
}

pub struct FakeFactory {
    steps: Mutex<VecDeque<ConnectStep>>,
    connections: Vec<Arc<dyn MultiplexedSshConnection>>,
    calls: AtomicUsize,
}

impl FakeFactory {
    pub fn empty(steps: Vec<ConnectStep>) -> Arc<Self> {
        Arc::new(Self {
            steps: Mutex::new(steps.into()),
            connections: Vec::new(),
            calls: AtomicUsize::new(0),
        })
    }

    pub fn new<C>(connections: Vec<Arc<C>>, steps: Vec<ConnectStep>) -> Arc<Self>
    where
        C: MultiplexedSshConnection + 'static,
    {
        Arc::new(Self {
            steps: Mutex::new(steps.into()),
            connections: connections
                .into_iter()
                .map(|connection| connection as Arc<dyn MultiplexedSshConnection>)
                .collect(),
            calls: AtomicUsize::new(0),
        })
    }
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::Relaxed)
    }
}

impl BrokerConnectionFactory for FakeFactory {
    fn connect_with_cleanup(&self, _: RemoteCleanupRegistry) -> BrokerConnectFuture<'_> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::Relaxed);
            match self
                .steps
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(ConnectStep::TransportFailure)
            {
                ConnectStep::Connection(index) => Ok(Arc::clone(&self.connections[index])),
                ConnectStep::TransportFailure => Err(RemoteError::transport("offline")),
                ConnectStep::AuthenticationFailure => {
                    Err(RemoteError::AuthenticationFailed("denied".into()))
                }
            }
        })
    }
}

pub fn test_config() -> BrokerConfig {
    BrokerConfig {
        hub_transport: false,
        host_id: HOST_ID.into(),
        profile_authority_sha256: PROFILE_AUTHORITY_SHA256.into(),
        endpoint: RemoteEndpoint::standard("builder", "build.example").unwrap(),
        known_hosts: KnownHosts::parse("build.example ssh-ed25519 encoded-key").unwrap(),
        identities: vec![],
        agent_socket: None,
        runtime: RemoteRuntimeSpec::new("/opt/yolo", RUNTIME_HOME).unwrap(),
        command_policy: CommandPolicy::new([ALLOWED_PROGRAM.into()]).unwrap(),
        file_policy: FileReadPolicy::standard(vec![PathBuf::from(READ_ROOT)]).unwrap(),
        artifact: None,
        revision: CONFIG_REVISION.into(),
    }
}

pub fn connection_info() -> ConnectionInfo {
    ConnectionInfo {
        connection_id: Uuid::new_v4(),
        endpoint: RemoteEndpoint::standard("builder", "build.example").unwrap(),
        host_key: HostKey::new("ssh-ed25519", "encoded-key").unwrap(),
    }
}
