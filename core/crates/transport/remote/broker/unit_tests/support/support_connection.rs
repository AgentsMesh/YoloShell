use super::support::{
    ARCHITECTURE_OUTPUT, PLATFORM_OUTPUT, READ_PATH, VERSION_OUTPUT, connection_info,
};
use super::support_channel::{OpenStep, open_channel};
use super::support_channel_probe::ChannelProbe;
use crate::remote::*;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use uuid::Uuid;
#[path = "../../test_support/unsupported_extensions.rs"]
mod unsupported_extensions;
use unsupported_extensions::unsupported_remote_extensions;
const COMPLETED_OPEN_INCREMENT: usize = 1;
pub struct FakeConnection {
    info: ConnectionInfo,
    next_channel: AtomicU64,
    opens: Mutex<Vec<Uuid>>,
    completed_opens: AtomicUsize,
    steps: Mutex<VecDeque<OpenStep>>,
    commands: AtomicUsize,
    files: AtomicUsize,
    probe: Arc<ChannelProbe>,
}
impl FakeConnection {
    pub fn new(steps: impl IntoIterator<Item = OpenStep>) -> Arc<Self> {
        Arc::new(Self {
            info: connection_info(),
            next_channel: AtomicU64::new(1),
            opens: Mutex::new(Vec::new()),
            completed_opens: AtomicUsize::new(usize::default()),
            steps: Mutex::new(steps.into_iter().collect()),
            commands: AtomicUsize::new(0),
            files: AtomicUsize::new(0),
            probe: ChannelProbe::new(),
        })
    }
    pub fn id(&self) -> Uuid {
        self.info.connection_id
    }
    pub fn opens(&self) -> Vec<Uuid> {
        self.opens.lock().unwrap().clone()
    }
    pub fn completed_opens(&self) -> usize {
        self.completed_opens.load(Ordering::Relaxed)
    }
    pub fn commands(&self) -> usize {
        self.commands.load(Ordering::Relaxed)
    }
    pub fn files(&self) -> usize {
        self.files.load(Ordering::Relaxed)
    }
    pub fn probe(&self) -> &ChannelProbe {
        &self.probe
    }
    fn identity(&self, purpose: ChannelPurpose) -> ChannelIdentity {
        ChannelIdentity {
            connection_id: self.id(),
            channel_id: self.next_channel.fetch_add(1, Ordering::Relaxed),
            purpose,
        }
    }
}

impl MultiplexedSshConnection for FakeConnection {
    unsupported_remote_extensions!();
    fn info(&self) -> &ConnectionInfo {
        &self.info
    }

    fn open_runtime<'a>(
        &'a self,
        request: &'a RuntimeChannelRequest,
    ) -> RemoteFuture<'a, OpenedRemoteChannel> {
        Box::pin(async move {
            self.opens.lock().unwrap().push(self.id());
            let step = self
                .steps
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or(OpenStep::Channel);
            if matches!(step, OpenStep::BlockedOpen) {
                self.probe.wait_open().await;
            }
            if matches!(step, OpenStep::TransportFailure) {
                return Err(RemoteError::transport("lost"));
            }
            if matches!(step, OpenStep::Rejected) {
                return Err(RemoteError::ChannelRejected);
            }
            let purpose = match request.role {
                RuntimeRole::Control => ChannelPurpose::RuntimeControl,
                RuntimeRole::Attachment => ChannelPurpose::TerminalAttachment,
            };
            let mut identity = self.identity(purpose);
            if matches!(step, OpenStep::Mismatch) {
                identity.connection_id = Uuid::nil();
            }
            if matches!(step, OpenStep::PurposeMismatch) {
                identity.purpose = ChannelPurpose::Command;
            }
            let stream = open_channel(step, identity, Arc::clone(&self.probe));
            self.completed_opens
                .fetch_add(COMPLETED_OPEN_INCREMENT, Ordering::Relaxed);
            Ok(OpenedRemoteChannel { identity, stream })
        })
    }

    fn execute<'a>(
        &'a self,
        command: &'a AuthorizedCommand,
    ) -> RemoteFuture<'a, ChannelOutcome<CommandOutput>> {
        Box::pin(async move {
            self.commands.fetch_add(1, Ordering::Relaxed);
            let stdout = match command.request().args().first().map(String::as_str) {
                Some(crate::remote::PLATFORM_ARGUMENT) => PLATFORM_OUTPUT.to_vec(),
                Some(crate::remote::ARCHITECTURE_ARGUMENT) => ARCHITECTURE_OUTPUT.to_vec(),
                Some(crate::remote::RUNTIME_VERSION_ARGUMENT) => VERSION_OUTPUT.to_vec(),
                _ => b"ok".to_vec(),
            };
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::Command),
                value: CommandOutput {
                    stdout,
                    stderr: vec![],
                    exit_status: Some(0),
                    timed_out: false,
                    truncated: false,
                },
            })
        })
    }

    fn canonicalize<'a>(&'a self, _: &'a Path) -> RemoteFuture<'a, ChannelOutcome<PathBuf>> {
        Box::pin(async move {
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::FileRead),
                value: PathBuf::from(READ_PATH),
            })
        })
    }

    fn read_file<'a>(
        &'a self,
        _: &'a AuthorizedFileRead,
    ) -> RemoteFuture<'a, ChannelOutcome<FileReadResult>> {
        Box::pin(async move {
            self.files.fetch_add(1, Ordering::Relaxed);
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::FileRead),
                value: FileReadResult {
                    resolved_path: PathBuf::from(READ_PATH),
                    data: b"data".to_vec(),
                    eof: true,
                },
            })
        })
    }

    fn stage_runtime<'a>(
        &'a self,
        _: &'a RuntimeStageRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<RuntimeStageResult>> {
        Box::pin(async move {
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::RuntimeDeployment),
                value: RuntimeStageResult {
                    already_active: true,
                    had_previous_runtime: true,
                },
            })
        })
    }
    fn activate_runtime<'a>(
        &'a self,
        _: &'a RuntimeActivationRequest,
    ) -> RemoteFuture<'a, ChannelOutcome<()>> {
        Box::pin(async move {
            Ok(ChannelOutcome {
                identity: self.identity(ChannelPurpose::RuntimeDeployment),
                value: (),
            })
        })
    }
}
