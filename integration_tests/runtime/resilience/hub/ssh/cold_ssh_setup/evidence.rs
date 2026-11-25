use super::{constants::*, fixture::Fixture};
use crate::hub::relay_crash::process::RelayProcess;
use std::{io::Read, path::PathBuf};
use yolo_distribution_publication_protocol::{CURRENT_FILE_NAME, Selection};
use yolo_hub_identity::{HubId, RuntimeTarget};
use yolo_hub_management_protocol::ConfigurationSnapshot;
use yolo_hub_remote_setup_protocol::{RemoteSetupOutcome, RemoteSetupRequest, RemoteSetupStatus};
use yolo_hub_remote_startup_protocol::RemoteBootstrapReceipt;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_state::HubState;
use yolo_remote_provisioning_protocol::ProvisionOutcome;
use yolo_runtime::paths::{identity::FileIdentity, private_file::PrivateFileEntry};
#[path = "routes.rs"]
mod routes;
pub use routes::{wait_hub, wait_route};

pub fn enrollment(status: &RemoteSetupStatus) -> RemoteBootstrapReceipt {
    let RemoteSetupOutcome::Linked { provision, .. } = &status.outcome else {
        panic!()
    };
    let ProvisionOutcome::Ready { enrollment, .. } = &provision.fields().outcome else {
        panic!()
    };
    enrollment.clone()
}

pub struct Evidence {
    files: Vec<FileProof>,
    selector: Selection,
    configuration: ConfigurationSnapshot,
    host: yolo_runtime::model::HostStatus,
    identities: (crate::ProcessIdentity, crate::ProcessIdentity),
}
impl Evidence {
    pub async fn capture(
        fixture: &Fixture,
        request: &RemoteSetupRequest,
        receipt: &RemoteBootstrapReceipt,
    ) -> Self {
        let id = request.request.fields().request_id.to_string();
        let files = vec![
            FileProof::capture(fixture.install.join(CURRENT_FILE_NAME)),
            FileProof::capture(
                fixture
                    .remote_state
                    .join(ENROLLMENT_JOURNAL)
                    .join(&id)
                    .join(RECORD_FILE),
            ),
            FileProof::capture(
                fixture
                    .broker
                    .registration
                    .broker_state
                    .join(BROKER_JOURNAL)
                    .join(&id)
                    .join(RECORD_FILE),
            ),
            FileProof::capture(
                fixture
                    .entry_state
                    .join(SETUP_JOURNAL)
                    .join(&id)
                    .join(RECORD_FILE),
            ),
        ];
        let selector: Selection = serde_json::from_slice(&files[0].bytes).unwrap();
        assert_eq!(selector.operation_id(), request.request.fields().request_id);
        assert_eq!(
            selector.build_id(),
            request.request.fields().source.descriptor().build_id()
        );
        let journal: serde_json::Value = serde_json::from_slice(&files[1].bytes).unwrap();
        let completion: RemoteBootstrapReceipt =
            serde_json::from_value(journal[JOURNAL_RECORD][JOURNAL_COMPLETION].clone()).unwrap();
        assert_eq!(&completion, receipt);
        let configuration = remote_configuration(fixture, receipt).await;
        assert_eq!(
            configuration.state.revision,
            receipt.fields().configuration_revision
        );
        assert_eq!(
            configuration.peer_socket.as_ref(),
            Some(&receipt.fields().peer_socket)
        );
        let host = fixture.direct_host().status().await.unwrap();
        let identities = fixture.owners.identities();
        assert_eq!(host.host_pid, identities.0.pid);
        Self {
            files,
            selector,
            configuration,
            host,
            identities,
        }
    }
    pub async fn verify(
        &self,
        fixture: &Fixture,
        request: &RemoteSetupRequest,
        receipt: &RemoteBootstrapReceipt,
    ) {
        for file in &self.files {
            file.verify();
        }
        let selector: Selection = serde_json::from_slice(
            &std::fs::read(fixture.install.join(CURRENT_FILE_NAME)).unwrap(),
        )
        .unwrap();
        assert_eq!(selector, self.selector);
        assert_eq!(selector.operation_id(), request.request.fields().request_id);
        assert_eq!(
            HubState::inspect_peer_identity(&fixture.remote_state, &fixture.home).unwrap(),
            Some(receipt.fields().peer)
        );
        assert_eq!(
            remote_configuration(fixture, receipt).await,
            self.configuration
        );
        let host = fixture.direct_host().status().await.unwrap();
        assert_eq!(host.host_pid, self.host.host_pid);
        assert_eq!(host.boot_id, self.host.boot_id);
        assert_eq!(host.controller_generation, self.host.controller_generation);
        assert!(self.identities.0.matches() && self.identities.1.matches());
        assert_eq!(fixture.owners.identities(), self.identities);
    }
}

struct FileProof {
    entry: PrivateFileEntry,
    identity: FileIdentity,
    bytes: Vec<u8>,
}
impl FileProof {
    fn capture(path: PathBuf) -> Self {
        let entry = PrivateFileEntry::open(&path).unwrap();
        let file = entry.open_read(MAX_JOURNAL_BYTES).unwrap();
        let mut bytes = Vec::new();
        file.file()
            .take(MAX_JOURNAL_BYTES)
            .read_to_end(&mut bytes)
            .unwrap();
        entry
            .verify_exact_identity(file.identity(), MAX_JOURNAL_BYTES)
            .unwrap();
        Self {
            entry,
            identity: file.identity(),
            bytes,
        }
    }
    fn verify(&self) {
        self.entry
            .verify_exact_identity(self.identity, MAX_JOURNAL_BYTES)
            .unwrap();
        let file = self.entry.open_read(MAX_JOURNAL_BYTES).unwrap();
        let mut bytes = Vec::new();
        file.file()
            .take(MAX_JOURNAL_BYTES)
            .read_to_end(&mut bytes)
            .unwrap();
        assert_eq!(bytes, self.bytes);
        self.entry
            .verify_exact_identity(self.identity, MAX_JOURNAL_BYTES)
            .unwrap();
    }
}

async fn remote_configuration(
    fixture: &Fixture,
    receipt: &RemoteBootstrapReceipt,
) -> ConfigurationSnapshot {
    let mut client = yolo_hub_client::Client::connect_local(
        &HubState::socket_path_for(&fixture.remote_state).unwrap(),
        receipt.fields().peer.hub(),
        deadline(),
    )
    .await
    .unwrap();
    let Reply::Configuration { snapshot } = client
        .call(
            receipt.fields().peer.hub(),
            Command::GetConfiguration {},
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    snapshot.validate().unwrap();
    snapshot
}
