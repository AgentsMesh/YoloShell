#[path = "support/server.rs"]
mod server;
#[path = "support/transport.rs"]
mod transport;

use super::super::constants::REQUIRED_RUNTIME_CAPABILITIES;
use super::super::*;
use crate::model::HostStatus;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::Notify;

pub(super) const TEST_CHANNEL_CAPACITY: usize = 65_536;
pub(super) const TEST_ATTACHMENT_AFTER_SEQUENCE: u64 = 6;
pub(super) const TEST_ATTACHMENT_NEXT_SEQUENCE: u64 = 7;

#[derive(Clone, Default)]
pub enum RuntimeBehavior {
    #[default]
    Normal,
    StallHello(Arc<Notify>),
    StallStatus(Arc<Notify>),
    StallAttachment(Arc<Notify>),
    DelayHelloThenStallStatus {
        delay: Duration,
        started: Arc<Notify>,
        reached: Arc<Notify>,
    },
    DelayOpenThenStallAttachment {
        delay: Duration,
        reached: Arc<Notify>,
    },
}

pub struct FakeConnection {
    info: ConnectionInfo,
    home: PathBuf,
    boot_id: String,
    generation: u64,
    next_channel: AtomicU64,
    purposes: Mutex<Vec<ChannelPurpose>>,
    requests: Arc<Mutex<Vec<String>>>,
    command_output: Mutex<CommandOutput>,
    command_outputs: Mutex<VecDeque<CommandOutput>>,
    resolved_path: Mutex<PathBuf>,
    file_result: Mutex<FileReadResult>,
    purpose_overrides: Mutex<VecDeque<ChannelPurpose>>,
    runtime_behaviors: Mutex<VecDeque<RuntimeBehavior>>,
    break_control_after_handshake: AtomicBool,
}

impl FakeConnection {
    pub fn new(endpoint: RemoteEndpoint, key: HostKey, home: PathBuf, boot_id: &str) -> Arc<Self> {
        Arc::new(Self {
            info: ConnectionInfo {
                connection_id: uuid::Uuid::new_v4(),
                endpoint,
                host_key: key,
            },
            home,
            boot_id: boot_id.into(),
            generation: 1,
            next_channel: AtomicU64::new(1),
            purposes: Mutex::new(Vec::new()),
            requests: Arc::default(),
            command_output: Mutex::new(CommandOutput {
                stdout: b"ok".to_vec(),
                stderr: vec![],
                exit_status: Some(0),
                timed_out: false,
                truncated: false,
            }),
            command_outputs: Mutex::new(VecDeque::new()),
            resolved_path: Mutex::new(PathBuf::from("/srv/project/file")),
            file_result: Mutex::new(FileReadResult {
                resolved_path: PathBuf::from("/srv/project/file"),
                data: b"data".to_vec(),
                eof: true,
            }),
            purpose_overrides: Mutex::new(VecDeque::new()),
            runtime_behaviors: Mutex::new(VecDeque::new()),
            break_control_after_handshake: AtomicBool::new(false),
        })
    }

    pub fn purposes(&self) -> Vec<ChannelPurpose> {
        self.purposes.lock().unwrap().clone()
    }

    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().unwrap().clone()
    }

    pub fn set_purpose_override(&self, value: ChannelPurpose) {
        self.set_purpose_overrides([value]);
    }

    pub fn set_purpose_overrides(&self, values: impl IntoIterator<Item = ChannelPurpose>) {
        *self.purpose_overrides.lock().unwrap() = values.into_iter().collect();
    }

    pub fn set_runtime_behaviors(&self, values: impl IntoIterator<Item = RuntimeBehavior>) {
        *self.runtime_behaviors.lock().unwrap() = values.into_iter().collect();
    }

    pub fn break_control_after_handshake(&self) {
        self.break_control_after_handshake
            .store(true, Ordering::Relaxed);
    }

    pub fn set_file_result(&self, value: FileReadResult) {
        *self.file_result.lock().unwrap() = value;
    }

    pub fn set_command_outputs(&self, values: impl IntoIterator<Item = CommandOutput>) {
        *self.command_outputs.lock().unwrap() = values.into_iter().collect();
    }

    fn identity(&self, expected: ChannelPurpose) -> ChannelIdentity {
        let purpose = self
            .purpose_overrides
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or(expected);
        self.purposes.lock().unwrap().push(expected);
        ChannelIdentity {
            connection_id: self.info.connection_id,
            channel_id: self.next_channel.fetch_add(1, Ordering::Relaxed),
            purpose,
        }
    }

    fn next_runtime_behavior(&self) -> RuntimeBehavior {
        self.runtime_behaviors
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_default()
    }

    fn status(&self) -> HostStatus {
        HostStatus {
            protocol_version: crate::model::PROTOCOL_VERSION,
            boot_id: self.boot_id.clone(),
            host_pid: 101,
            controller_generation: self.generation,
            home: self.home.clone(),
            capabilities: REQUIRED_RUNTIME_CAPABILITIES
                .iter()
                .map(ToString::to_string)
                .collect(),
        }
    }
}

pub fn known_hosts() -> KnownHosts {
    KnownHosts::parse("build.example ssh-ed25519 encoded-key").unwrap()
}

pub fn connection(home: &str, boot: &str) -> Arc<FakeConnection> {
    FakeConnection::new(
        RemoteEndpoint::standard("stone", "build.example").unwrap(),
        HostKey::new("ssh-ed25519", "encoded-key").unwrap(),
        PathBuf::from(home),
        boot,
    )
}
