use yolo_hub_management_protocol::{
    CONFIGURATION_SCHEMA, ConfigurationSnapshot, ConfigurationState, FIRST_CONFIGURATION_REVISION,
};
use yolo_hub_remote_setup_protocol::RemoteSetupRequest;
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionStatus};

pub fn request() -> RemoteSetupRequest {
    RemoteSetupRequest {
        request: yolo_remote_provisioning_protocol_test_support::call().request,
        expected_configuration_revision: FIRST_CONFIGURATION_REVISION,
    }
}
pub fn baseline(request: &RemoteSetupRequest) -> ConfigurationSnapshot {
    ConfigurationSnapshot {
        state: ConfigurationState {
            schema_version: CONFIGURATION_SCHEMA,
            identity: request.request.fields().bootstrap.fields().consumer,
            revision: request.expected_configuration_revision,
            last_change: uuid::Uuid::new_v4(),
            configuration: Default::default(),
        },
        applied: true,
        listen_address: None,
    }
}
pub fn published(request: &RemoteSetupRequest) -> ProvisionStatus {
    let fields = request.request.fields();
    let publication = yolo_distribution_publication_protocol::PublicationRequest::new(
        fields.install_root.clone(),
        fields.request_id,
        fields.request_id,
        fields.source.descriptor().clone(),
        None,
    )
    .unwrap();
    let receipt = yolo_distribution_publication_protocol::PublicationReceipt::new(
        &publication,
        &fields.install_root,
        publication.selected().unwrap(),
    )
    .unwrap();
    ProvisionStatus::new(
        request.request.clone(),
        ProvisionOutcome::Published {
            publication,
            receipt,
        },
    )
    .unwrap()
}
pub fn ready(request: &RemoteSetupRequest) -> ProvisionStatus {
    let ProvisionOutcome::Published {
        publication,
        receipt,
    } = published(request).fields().outcome.clone()
    else {
        panic!()
    };
    let mut enrollment =
        yolo_hub_remote_startup_test_support::receipt(&request.request.fields().bootstrap);
    enrollment.peer = request
        .request
        .fields()
        .bootstrap
        .fields()
        .expected_peer
        .unwrap();
    ProvisionStatus::new(
        request.request.clone(),
        ProvisionOutcome::Ready {
            publication,
            receipt,
            enrollment: yolo_remote_provisioning_protocol::RemoteBootstrapReceipt::new(enrollment)
                .unwrap(),
        },
    )
    .unwrap()
}
