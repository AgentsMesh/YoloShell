use super::{fixture, hub_readiness_server::*, hub_start_fixture::*};
use crate::start::reconcile;
use std::sync::atomic::Ordering;
use yolo_remote_broker::BrokerRequest;

#[tokio::test]
async fn pending_hub_readiness_prepares_ssh_and_hands_off_without_legacy_bootstrap() {
    let fixture = fixture();
    let spawner = PendingSpawner::new(Fault::None);
    let ready = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    assert!(ready.connection_id.is_some());
    assert_eq!(ready.connection_generation, GENERATION);
    assert_eq!(spawner.calls.load(Ordering::SeqCst), EXPECTED_CONNECTIONS);
    assert_eq!(spawner.compensated.load(Ordering::SeqCst), usize::default());
    let servers = spawner.servers.lock().unwrap();
    let requests = servers[0].requests.lock().unwrap();
    assert!(matches!(
        requests.as_slice(),
        [BrokerRequest::Status, BrokerRequest::PrepareHubTransport(_)]
    ));
}

#[tokio::test]
async fn same_revision_hub_is_reused_with_only_status_and_typed_preparation() {
    let fixture = fixture();
    let server = WireBroker::bind(&fixture.options, true, Fault::None);
    let spawner = NoSpawn::default();
    let first = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    let second = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap();
    assert_eq!(first, second);
    assert!(first.adopted);
    assert_eq!(spawner.0.load(Ordering::SeqCst), usize::default());
    let requests = server.requests.lock().unwrap();
    assert!(matches!(
        requests.as_slice(),
        [
            BrokerRequest::Status,
            BrokerRequest::PrepareHubTransport(_),
            BrokerRequest::Status,
            BrokerRequest::PrepareHubTransport(_)
        ]
    ));
}

#[tokio::test]
async fn pending_wrong_instance_or_profile_is_compensated_instead_of_adopted() {
    for fault in [
        Fault::Instance,
        Fault::Profile,
        Fault::Authority,
        Fault::Revision,
    ] {
        let fixture = fixture();
        let spawner = PendingSpawner::new(fault);
        let error = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
            .await
            .unwrap_err();
        if matches!(fault, Fault::Instance) {
            assert!(error.to_string().contains(IDENTITY_MISMATCH));
        }
        assert_eq!(spawner.calls.load(Ordering::SeqCst), EXPECTED_CONNECTIONS);
        assert_eq!(
            spawner.compensated.load(Ordering::SeqCst),
            EXPECTED_CONNECTIONS
        );
        let servers = spawner.servers.lock().unwrap();
        let requests = servers[0].requests.lock().unwrap();
        assert!(matches!(
            requests.as_slice(),
            [BrokerRequest::Status, BrokerRequest::PrepareHubTransport(_)]
        ));
    }
}

#[tokio::test]
async fn initial_status_cannot_name_another_host_even_with_a_correct_prepare_binding() {
    let fixture = fixture();
    let server = WireBroker::bind(&fixture.options, true, Fault::StatusHost);
    let spawner = NoSpawn::default();
    let error = reconcile(fixture.options.clone(), &spawner, fixture::START_TIMEOUT)
        .await
        .unwrap_err();
    assert!(error.to_string().contains(IDENTITY_MISMATCH));
    assert_eq!(spawner.0.load(Ordering::SeqCst), usize::default());
    assert!(matches!(
        server.requests.lock().unwrap().as_slice(),
        [BrokerRequest::Status]
    ));
}
