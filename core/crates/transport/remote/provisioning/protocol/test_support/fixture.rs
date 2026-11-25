use super::constants::*;
use yolo_broker_hub_protocol::{BrokerHubConnectionBinding, BrokerHubProfile};
use yolo_distribution_bundle::BundleSource;
use yolo_hub_remote_startup_test_support::{RUNTIME_DIGEST, request};
use yolo_remote_provisioning_protocol::*;
pub fn call() -> BrokerProvisionCall {
    let bootstrap = request();
    let profile = BrokerHubProfile {
        host_id: PROFILE.into(),
        profile_authority_sha256: RUNTIME_DIGEST.into(),
        config_revision: RUNTIME_DIGEST.into(),
    };
    BrokerProvisionCall {
        binding: BrokerHubConnectionBinding {
            profile: profile.clone(),
            broker_instance_id: uuid::Uuid::new_v4(),
            connection_id: uuid::Uuid::new_v4(),
            connection_generation: 1,
        },
        request: ProvisionRequest::new(ProvisionFields {
            schema_version: PROVISION_SCHEMA_VERSION,
            request_id: bootstrap.fields().request_id,
            profile,
            source: BundleSource::new(ROOT.into(), bootstrap.fields().bundle.clone()).unwrap(),
            install_root: INSTALL.into(),
            state: STATE.into(),
            host_home: HOME.into(),
            bootstrap,
        })
        .unwrap(),
    }
}
