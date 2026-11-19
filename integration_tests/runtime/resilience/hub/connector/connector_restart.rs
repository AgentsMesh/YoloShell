use super::{constants::*, graph, observe, process::Publisher, requests, shell};
use crate::hub::federation_support::TEMP_ROOT;
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_connector_protocol::{
    Command, ConnectorCredential, ConnectorError, EnrollmentStatus, Reply,
};
use yolo_hub_identity::ConnectorInstanceId;

#[tokio::test(flavor = "multi_thread")]
async fn real_connector_registry_restart_requires_reauthentication_and_preserves_shell() {
    let harness = crate::agent_recovery::federation_harness().await;
    let root = tempfile::tempdir_in(TEMP_ROOT).unwrap();
    let path = root.path().canonicalize().unwrap().join(C_STATE);
    let node = graph::standalone(&harness, &path).await;
    let hub = node.running().node.hub_id();
    let peer = node.identity;
    let target = shell::create(node.running(), &harness).await;
    let before = harness
        .client()
        .get_session(target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let enrollment = requests::enroll(&node, target).await;
    let instance = ConnectorInstanceId::new();
    let mut publisher = Publisher::start(&node.socket, hub).await;
    let active = requests::activate_publisher(&mut publisher, &enrollment, instance).await;
    let first = requests::publish(
        &mut publisher,
        requests::publication(active, active.next_sequence),
    )
    .await;
    let original = observe::agent(node.running(), enrollment.agent, Freshness::Current).await;
    // Stop services while their publisher remains alive: reopening reconstructs only stale facts.
    node.stop().await;
    publisher.crash().await;
    let node = graph::standalone(&harness, &path).await;
    assert_eq!(node.running().node.hub_id(), hub);
    assert_eq!(node.identity, peer);
    let reopened = requests::status(&node, &enrollment).await;
    assert_eq!(reopened.agent, enrollment.agent);
    assert_eq!(reopened.scope, enrollment.scope);
    assert_eq!(reopened.status, EnrollmentStatus::Enabled);
    let stale = observe::agent(node.running(), enrollment.agent, Freshness::Stale).await;
    observe::same(&original, &stale);
    assert_eq!(stale.fields().reachability, Reachability::Unreachable);
    shell::survives(&harness, target, &before).await;
    shell::ping(node.running(), target).await;
    let mut publisher = Publisher::start(&node.socket, hub).await;
    let invalid = Command::Activate {
        connector_id: enrollment.connector_id,
        credential: ConnectorCredential::generate(),
        instance_id: instance,
    };
    assert_eq!(
        publisher.call(invalid).await,
        Reply::Error {
            code: ConnectorError::Unauthorized
        }
    );
    observe::agent(node.running(), enrollment.agent, Freshness::Stale).await;
    let current = requests::activate_publisher(&mut publisher, &enrollment, instance).await;
    assert_eq!(current.agent, active.agent);
    assert_ne!(current.generation, active.generation);
    let ack = requests::publish(
        &mut publisher,
        requests::publication(current, current.next_sequence),
    )
    .await;
    assert_ne!(ack.cursor.stream(), first.cursor.stream());
    let restored = observe::at(node.running(), ack.cursor).await;
    observe::bound(node.running(), &restored).await;
    shell::survives(&harness, target, &before).await;
    publisher.stop().await;
    node.stop().await;
}
