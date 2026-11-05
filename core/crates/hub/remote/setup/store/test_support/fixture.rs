use super::constants::*;
use yolo_broker_hub_protocol::BrokerHubProfile;
use yolo_distribution_bundle::BundleSource;
use yolo_distribution_publication_protocol::{PublicationReceipt, PublicationRequest};
use yolo_hub_federation_settings::Configuration;
use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState,
};
use yolo_hub_remote_setup_protocol::{RemoteSetupRequest, plan_link};
use yolo_hub_remote_setup_store::{OpenedSetup, SetupOperation, SetupPhase, SetupRecord};
use yolo_remote_provisioning_protocol::RemoteBootstrapReceipt;
use yolo_hub_remote_startup_test_support::{RUNTIME_DIGEST, receipt, request};
use yolo_hub_ssh_profile_protocol::SshBrokerProfile;
use yolo_paths::{directory::PrivateDirectory, identity::DurableObjectIdentity};
use yolo_remote_provisioning_protocol::{
    PROVISION_SCHEMA_VERSION, ProvisionFields, ProvisionOutcome, ProvisionRequest, ProvisionStatus,
};

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub parent: PrivateDirectory,
    pub record: SetupRecord,
    pub broker: DurableObjectIdentity,
}
impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let parent = PrivateDirectory::ensure(&root.path().join(SETUP_NAME)).unwrap();
        let broker_directory = PrivateDirectory::ensure(&root.path().join(BROKER_NAME)).unwrap();
        let bootstrap = request();
        let profile = BrokerHubProfile {
            host_id: PROFILE_ID.into(),
            profile_authority_sha256: RUNTIME_DIGEST.into(),
            config_revision: RUNTIME_DIGEST.into(),
        };
        let request = RemoteSetupRequest {
            expected_configuration_revision: FIRST_REVISION,
            request: ProvisionRequest::new(ProvisionFields {
                schema_version: PROVISION_SCHEMA_VERSION,
                request_id: bootstrap.fields().request_id,
                profile,
                source: BundleSource::new(SOURCE_ROOT.into(), bootstrap.fields().bundle.clone())
                    .unwrap(),
                install_root: INSTALL_ROOT.into(),
                state: REMOTE_STATE.into(),
                host_home: REMOTE_HOME.into(),
                bootstrap,
            })
            .unwrap(),
        };
        let owner = request.request.fields().bootstrap.fields().consumer;
        let registration = SshBrokerProfile {
            profile: request.profile().unwrap(),
            broker_state: broker_directory.path().to_owned(),
            config_revision: RUNTIME_DIGEST.into(),
            launch: None,
        };
        let baseline = ConfigurationSnapshot {
            state: ConfigurationState {
                schema_version: CONFIGURATION_SCHEMA,
                identity: owner,
                revision: FIRST_REVISION,
                last_change: uuid::Uuid::new_v4(),
                configuration: Configuration::default(),
            },
            applied: true,
            peer_socket: Some("/run/user/1000/yolo/hub/peer.sock".into()),
        };
        let record = SetupRecord::new(owner, request, registration, baseline).unwrap();
        Self {
            root,
            parent,
            record,
            broker: broker_directory.durable_identity().unwrap(),
        }
    }
    pub fn id(&self) -> uuid::Uuid {
        self.record.request.request.fields().request_id
    }
    pub fn file(&self) -> std::path::PathBuf {
        self.parent
            .path()
            .join(self.id().to_string())
            .join(RECORD_NAME)
    }
    pub fn open(&self) -> OpenedSetup {
        SetupOperation::open(self.parent.try_clone().unwrap(), self.record.clone()).unwrap()
    }
    pub fn fresh(&self) -> SetupOperation {
        let OpenedSetup::Fresh(operation) = self.open() else {
            panic!("expected fresh operation")
        };
        operation
    }
    pub fn provision(&self) -> ProvisionStatus {
        let request = &self.record.request.request;
        let fields = request.fields();
        let publication = PublicationRequest::new(
            &fields.install_root,
            fields.request_id,
            fields.request_id,
            fields.source.descriptor().clone(),
            None,
        )
        .unwrap();
        let selected = publication.selected().unwrap();
        let publication_receipt =
            PublicationReceipt::new(&publication, &fields.install_root, selected).unwrap();
        let enrollment = RemoteBootstrapReceipt::new(receipt(&fields.bootstrap)).unwrap();
        ProvisionStatus::new(
            request.clone(),
            ProvisionOutcome::Ready {
                publication,
                receipt: publication_receipt,
                enrollment,
            },
        )
        .unwrap()
    }
    pub fn routing(&self) -> SetupPhase {
        let provision = self.provision();
        let change = plan_link(&self.record.baseline, &self.record.request, &provision).unwrap();
        SetupPhase::Routing {
            broker_directory: self.broker,
            change,
            provision,
        }
    }
    pub fn complete(&self) -> SetupPhase {
        let SetupPhase::Routing {
            broker_directory,
            change,
            provision,
        } = self.routing()
        else {
            unreachable!()
        };
        SetupPhase::Complete {
            broker_directory,
            change,
            provision,
        }
    }
    pub fn other_directory(&self) -> DurableObjectIdentity {
        PrivateDirectory::ensure(&self.root.path().join(FOREIGN_NAME))
            .unwrap()
            .durable_identity()
            .unwrap()
    }
}
