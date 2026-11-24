use super::constants::*;
use std::sync::{Arc, Mutex};
use yolo_remote_provisioning_protocol::*;
use yolo_remote::*;
use yolo_remote_bundle_test_support::fake::{Fake as BundleFake, Fault};
#[path = "fake_transport.rs"]
mod transport;

pub struct Fake {
    pub bundle: Arc<BundleFake>,
    pub state: Arc<Mutex<State>>,
    info: ConnectionInfo,
}
pub struct State {
    pub calls: usize,
    pub inspections: usize,
    pub lose_reply: bool,
    pub superseded: bool,
    pub receipt: Option<RemoteBootstrapReceipt>,
    pub gate: Option<(Arc<tokio::sync::Semaphore>, Arc<tokio::sync::Semaphore>)>,
}
impl Fake {
    pub fn new() -> Self {
        let bundle = Arc::new(BundleFake::new(Fault::None));
        Self {
            info: bundle.info().clone(),
            bundle,
            state: Arc::new(Mutex::new(State {
                calls: 0,
                inspections: 0,
                lose_reply: false,
                superseded: false,
                receipt: None,
                gate: None,
            })),
        }
    }
    pub fn reconnect(&self) -> Self {
        let mut info = self.info.clone();
        info.connection_id = uuid::Uuid::new_v4();
        Self {
            info,
            bundle: self.bundle.clone(),
            state: self.state.clone(),
        }
    }
    fn rebind<T>(&self, mut value: ChannelOutcome<T>) -> ChannelOutcome<T> {
        value.identity.connection_id = self.info.connection_id;
        value
    }
    async fn command(
        &self,
        command: &AuthorizedCommand,
    ) -> Result<ChannelOutcome<CommandOutput>, RemoteError> {
        let prefix = yolo_launch_contract::ProcessRole::Hub.arguments();
        let arguments = command.request().args();
        let action = arguments
            .strip_prefix(&prefix.map(str::to_owned))
            .and_then(|arguments| arguments.first())
            .map(String::as_str);
        if !matches!(
            action,
            Some(REMOTE_BOOTSTRAP_ARGUMENT | REMOTE_BOOTSTRAP_RECOVERY_ARGUMENT)
        ) {
            return self
                .bundle
                .execute(command)
                .await
                .map(|value| self.rebind(value));
        }
        let encoded = command
            .request()
            .args()
            .windows(2)
            .find(|pair| pair[0] == yolo_distribution_publication_protocol::REQUEST_CAPNP_ARGUMENT)
            .unwrap();
        let request: RemoteBootstrapRequest = yolo_capnp_argument::decode(
            &encoded[1],
            yolo_remote_provisioning_protocol::MAX_BOOTSTRAP_DOCUMENT_BYTES,
        )
        .unwrap();
        let mut receipt = yolo_hub_remote_startup_test_support::receipt(&request);
        receipt.peer = request.fields().expected_peer.unwrap();
        let image = |role| {
            let value = request.fields().bundle.artifact(role);
            yolo_build_image::BuildImage::new(value.sha256().into(), value.size_bytes()).unwrap()
        };
        receipt.build.host = image(yolo_distribution_bundle::ArtifactRole::Runtime);
        receipt.build.hub = image(yolo_distribution_bundle::ArtifactRole::Runtime);
        receipt.build.file_helper = image(yolo_distribution_bundle::ArtifactRole::Runtime);
        let receipt = RemoteBootstrapReceipt::new(receipt).unwrap();
        let (bytes, gate, lost) = {
            let mut state = self.state.lock().unwrap();
            if action == Some(REMOTE_BOOTSTRAP_ARGUMENT) {
                state.calls += 1;
                state.receipt = Some(receipt.clone());
                (
                    yolo_capnp_codec::encode(&receipt).unwrap(),
                    state.gate.take(),
                    state.lose_reply,
                )
            } else {
                state.inspections += 1;
                let outcome = if state.superseded {
                    EnrollmentOutcome::Superseded {}
                } else if let Some(receipt) = state.receipt.clone() {
                    EnrollmentOutcome::Ready { receipt }
                } else {
                    EnrollmentOutcome::Absent {}
                };
                (
                    yolo_capnp_codec::encode(&EnrollmentRecovery::new(&request, outcome).unwrap())
                        .unwrap(),
                    None,
                    false,
                )
            }
        };
        if let Some((entered, release)) = gate {
            entered.add_permits(ONE);
            release.acquire().await.unwrap().forget();
        }
        if lost {
            return Err(RemoteError::Transport(LOST.into()));
        }
        Ok(ChannelOutcome {
            identity: ChannelIdentity {
                connection_id: self.info.connection_id,
                channel_id: 1,
                purpose: ChannelPurpose::Command,
            },
            value: CommandOutput {
                stdout: bytes,
                stderr: Vec::new(),
                exit_status: Some(SUCCESS),
                timed_out: false,
                truncated: false,
            },
        })
    }
}
