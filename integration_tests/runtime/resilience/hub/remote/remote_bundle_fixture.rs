use super::{
    assets,
    constants::*,
    owners::Owners,
    process::{self, Invocation},
};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
use uuid::Uuid;
use yolo_distribution_bundle::BundleDescriptor;
use yolo_distribution_publication_protocol::{PublicationReceipt, Selection};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_local_startup::ConnectionDocument;
use yolo_launch_contract::{HOST_HOME_ARGUMENT, HUB_STATE_ARGUMENT};
use yolo_remote_provisioning_protocol::{
    REMOTE_BOOTSTRAP_SCHEMA_VERSION, RemoteBootstrapReceipt, RemoteBootstrapRequest,
    RemoteBootstrapRequestFields,
};
use yolo_hub_routing::Right;
use yolo_hub_state::HubState;

pub struct Fixture {
    owners: Owners,
    pub install: PathBuf,
    pub home: PathBuf,
    pub state: PathBuf,
    pub consumer: PeerIdentity,
    selected: Option<Selection>,
    root: tempfile::TempDir,
}
impl Fixture {
    pub fn new(existing_home: Option<&Path>) -> Self {
        let root = tempfile::tempdir().unwrap();
        let base = root.path().canonicalize().unwrap();
        let install = base.join(ROOT_NAME);
        let state = base.join(STATE_NAME);
        let home = existing_home
            .map(Path::to_path_buf)
            .unwrap_or_else(|| base.join(HOME_NAME));
        crate::prepare_host_home(&home);
        let consumer = HubState::open(&base.join(CONSUMER_STATE), &base.join(CONSUMER_HOME))
            .unwrap()
            .peer_identity();
        let codex = base.join(CODEX_DIRECTORY);
        assets::directory(&codex);
        std::fs::write(codex.join(CODEX_CONFIGURATION), CODEX_CONFIGURATION_BYTES).unwrap();
        Self {
            owners: Owners::new(&home, &state, existing_home.is_none()),
            install,
            home,
            state,
            consumer,
            selected: None,
            root,
        }
    }

    pub async fn install(&mut self, replace_runtime: bool) -> BundleDescriptor {
        let request = assets::stage(&self.install, self.selected.clone(), replace_runtime);
        let mut command = self.command(INSTALL_BUNDLE_ARGUMENT);
        command.arg(REQUEST_CAPNP_ARGUMENT).arg(
            yolo_distribution_publication_protocol::encode_request_argument(&request).unwrap(),
        );
        let result = process::invoke(&mut command, self.root.path(), None).await;
        result.success();
        let receipt: PublicationReceipt =
            yolo_distribution_publication_protocol::decode_receipt(&result.output).unwrap();
        receipt.validate(&request, &self.install).unwrap();
        assert_eq!(
            receipt.release_directory(),
            request.descriptor().release_directory(&self.install)
        );
        self.selected = Some(receipt.selection().clone());
        request.descriptor().clone()
    }

    pub fn request(
        &self,
        bundle: BundleDescriptor,
        peer: Option<PeerIdentity>,
    ) -> RemoteBootstrapRequest {
        RemoteBootstrapRequest::new(RemoteBootstrapRequestFields {
            schema_version: REMOTE_BOOTSTRAP_SCHEMA_VERSION,
            request_id: Uuid::new_v4(),
            bundle,
            expected_peer: peer,
            consumer: self.consumer,
            rights: [
                Right::Observe,
                Right::TerminalRead,
                Right::TerminalWrite,
                Right::Lifecycle,
            ]
            .into_iter()
            .collect(),
            remaining_reexports: REEXPORTS,
        })
        .unwrap()
    }

    pub async fn invoke(&mut self, request: &RemoteBootstrapRequest) -> Invocation {
        let mut command = self.command(REMOTE_BOOTSTRAP_ARGUMENT);
        command
            .arg(HUB_STATE_ARGUMENT)
            .arg(&self.state)
            .arg(HOST_HOME_ARGUMENT)
            .arg(&self.home)
            .arg(REQUEST_CAPNP_ARGUMENT)
            .arg(
                yolo_capnp_argument::encode(
                    request,
                    yolo_remote_provisioning_protocol::MAX_BOOTSTRAP_DOCUMENT_BYTES,
                )
                .unwrap(),
            );
        process::invoke(&mut command, self.root.path(), Some(&mut self.owners)).await
    }

    pub async fn bootstrap(&mut self, request: &RemoteBootstrapRequest) -> RemoteBootstrapReceipt {
        let result = self.invoke(request).await;
        result.success();
        let receipt: RemoteBootstrapReceipt = yolo_capnp_codec::decode(&result.output).unwrap();
        receipt.validate_for(request).unwrap();
        assert_eq!(
            HubState::inspect_peer_identity(&self.state, &self.home).unwrap(),
            Some(receipt.fields().peer)
        );
        assert!(receipt.fields().peer_socket.is_absolute());
        let (host, hub) = self.owners.identities();
        assert!(host.matches() && hub.matches());
        receipt
    }

    pub fn connection(&self, peer: PeerIdentity) -> ConnectionDocument {
        ConnectionDocument::local(peer.hub(), HubState::socket_path_for(&self.state).unwrap())
    }

    pub fn identities(&self) -> (crate::ProcessIdentity, crate::ProcessIdentity) {
        self.owners.identities()
    }

    pub async fn finish(&mut self) {
        self.owners.stop().await;
    }

    fn command(&self, action: &str) -> Command {
        let mut command = Command::new(crate::hub::local_startup_support::hub_binary());
        command
            .args(yolo_launch_contract::ProcessRole::Hub.arguments())
            .arg(action)
            .arg(INSTALL_ROOT_ARGUMENT)
            .arg(&self.install)
            .env(crate::SHELL_ENV, crate::TEST_SHELL)
            .env(
                crate::CODEX_HOME_ENV,
                self.root.path().join(CODEX_DIRECTORY),
            );
        command
    }
}
