use super::super::*;
use super::support::{ConnectStep, FakeConnection, FakeFactory, test_config};
use crate::remote_broker::provisioning_rpc::Action;
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionRequest};

#[tokio::test]
async fn provisioning_requires_hub_mode_exact_profile_instance_and_cached_generation() {
    let mut call = yolo_remote_provisioning_protocol_test_support::call();
    let mut config = test_config();
    let factory = FakeFactory::empty(Vec::new());
    let pool = BrokerConnectionPool::configured(&config, factory.clone());
    let allowed = || Ok(());
    assert!(
        pool.provision_hub(
            &call,
            Action::Provision,
            tokio::time::Instant::now() + timing::BROKER_CLIENT_OPERATION_TIMEOUT,
            &allowed
        )
        .await
        .is_err()
    );
    assert_eq!(factory.calls(), 0);
    config.hub_transport = true;
    config.revision = yolo_broker_hub_test_support::HASH.into();
    let factory = FakeFactory::new(
        vec![FakeConnection::new([])],
        vec![ConnectStep::Connection(0)],
    );
    let mut pool = BrokerConnectionPool::configured(&config, factory.clone());
    let profile = yolo_broker_hub_protocol::BrokerHubProfile {
        host_id: config.host_id().into(),
        profile_authority_sha256: config.profile_authority_sha256().into(),
        config_revision: config.revision().into(),
    };
    let mut fields = call.request.fields().clone();
    fields.profile = profile.clone();
    call.request = ProvisionRequest::new(fields).unwrap();
    call.binding.profile = profile.clone();
    assert!(
        pool.provision_hub(
            &call,
            Action::Provision,
            tokio::time::Instant::now() + timing::BROKER_CLIENT_OPERATION_TIMEOUT,
            &allowed
        )
        .await
        .is_err()
    );
    assert_eq!(factory.calls(), 0); // Provision never establishes a replacement SSH connection.
    let binding = pool.prepare_hub_transport(&profile).await.unwrap();
    let root = tempfile::tempdir().unwrap();
    let paths = BrokerPaths::new(root.path()).unwrap();
    pool.retain_provisioning_paths(&paths);
    call.binding = binding;
    let mut stale = call.clone();
    stale.binding.connection_generation += 1;
    let deadline = tokio::time::Instant::now() + timing::BROKER_CLIENT_OPERATION_TIMEOUT;
    assert!(
        pool.provision_hub(&stale, Action::Provision, deadline, &allowed)
            .await
            .is_err()
    );
    let recovered = pool
        .provision_hub(&call, Action::Recover, deadline, &allowed)
        .await
        .unwrap();
    assert!(matches!(
        recovered.fields().outcome,
        ProvisionOutcome::Absent {}
    ));
    assert!(
        !root
            .path()
            .join(yolo_remote_provisioning::PROVISIONING_DIRECTORY)
            .exists()
    );
    let observed = pool.observed_connection().await.unwrap();
    pool.invalidate(&observed).await;
    assert!(
        pool.provision_hub(&call, Action::Recover, deadline, &allowed)
            .await
            .is_err()
    );
    assert_eq!(factory.calls(), 1);
}

#[test]
fn pending_adoption_never_accepts_any_provisioning_operation() {
    let call = yolo_remote_provisioning_protocol_test_support::call();
    for request in [
        BrokerRequest::ProvisionHub(call.clone()),
        BrokerRequest::RecoverHubProvision(call.clone()),
        BrokerRequest::AdvanceHubProvision(call),
    ] {
        assert!(!server_admission::permits_pending_request(&request));
    }
}
