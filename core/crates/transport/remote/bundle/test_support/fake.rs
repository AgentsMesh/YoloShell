use super::constants::*;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};
use uuid::Uuid;
use yolo_distribution_bundle::ArtifactRole;
use yolo_distribution_publication_protocol::*;
use yolo_remote::*;
#[path = "fake_command.rs"]
mod command;
#[path = "fake_transport.rs"]
mod transport;

#[derive(Clone, Copy)]
pub enum Fault {
    None,
    WrongPlatform,
    WrongArchitecture,
    WrongVersion(ArtifactRole),
    UnverifiedStaged(ArtifactRole),
    WrongVerificationChannel,
    Timeout,
    Truncated,
    MissingExit,
    WrongChannel(ChannelPurpose),
    StageFailure,
    WrongStage,
    LostAck,
    Superseded,
    FailedCommit,
    InvalidReceipt,
    CorruptRecovery,
    MalformedSelection,
}

pub struct Fake {
    info: ConnectionInfo,
    sequence: AtomicU64,
    state: Mutex<State>,
}

struct State {
    fault: Fault,
    selected: Option<Selection>,
    staged: Option<BundleStageRequest>,
    committed: Option<PublicationRequest>,
    files: BTreeMap<PathBuf, Vec<u8>>,
    commands: Vec<CommandRequest>,
    stages: usize,
    verifications: usize,
}

impl Fake {
    pub fn new(fault: Fault) -> Self {
        Self {
            info: ConnectionInfo {
                connection_id: Uuid::new_v4(),
                endpoint: RemoteEndpoint::standard(USER, HOST).unwrap(),
                host_key: HostKey::new(KEY_KIND, KEY_DATA).unwrap(),
            },
            sequence: AtomicU64::new(0),
            state: Mutex::new(State {
                fault,
                selected: None,
                staged: None,
                committed: None,
                files: BTreeMap::new(),
                commands: Vec::new(),
                stages: 0,
                verifications: 0,
            }),
        }
    }
    pub fn commits(&self) -> usize {
        self.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|command| {
                command
                    .args()
                    .strip_prefix(
                        &yolo_launch_contract::ProcessRole::Hub
                            .arguments()
                            .map(str::to_owned),
                    )
                    .and_then(|arguments| arguments.first())
                    .is_some_and(|value| value == INSTALL_BUNDLE_ARGUMENT)
            })
            .count()
    }
    pub fn inspections(&self) -> usize {
        self.state
            .lock()
            .unwrap()
            .commands
            .iter()
            .filter(|command| {
                command
                    .args()
                    .strip_prefix(
                        &yolo_launch_contract::ProcessRole::Hub
                            .arguments()
                            .map(str::to_owned),
                    )
                    .and_then(|arguments| arguments.first())
                    .is_some_and(|value| value == INSPECT_INSTALL_ARGUMENT)
            })
            .count()
    }
    pub fn stages(&self) -> usize {
        self.state.lock().unwrap().stages
    }
    pub fn verifications(&self) -> usize {
        self.state.lock().unwrap().verifications
    }
    pub fn set_selected(&self, selected: Option<Selection>) {
        self.state.lock().unwrap().selected = selected;
    }
    pub fn commands(&self) -> Vec<CommandRequest> {
        self.state.lock().unwrap().commands.clone()
    }
    fn identity(&self, purpose: ChannelPurpose) -> ChannelIdentity {
        let wrong = matches!(self.state.lock().unwrap().fault, Fault::WrongChannel(expected) if expected == purpose);
        ChannelIdentity {
            connection_id: if wrong {
                Uuid::new_v4()
            } else {
                self.info.connection_id
            },
            channel_id: self.sequence.fetch_add(1, Ordering::SeqCst),
            purpose,
        }
    }
}

fn output(bytes: &[u8]) -> CommandOutput {
    CommandOutput {
        stdout: bytes.into(),
        stderr: Vec::new(),
        exit_status: Some(SUCCESS),
        timed_out: false,
        truncated: false,
    }
}
