use super::{
    cleanup_rpc::{ProtocolFixture, bounded},
    connector_barrier::{accept_while, assert_pending},
    connectors as support,
    fixtures::{target, view},
};
use yolo_hub_connector_protocol::ConnectorError;
use yolo_hub_identity::ConnectorInstanceId;

#[tokio::test]
async fn older_unavailable_observation_retires_old_publication_before_its_host_read() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let shell = view();
    let enrollment = support::register(&fixture, &shell).await;
    let active = support::activate(&fixture, &enrollment, &shell).await;
    support::publish(&fixture, active, &shell).await;
    let observer = fixture.backend.clone();
    let mut older = Box::pin(observer.observe_agents());
    let (mut blocked, request) = accept_while(&fixture, &mut older).await;
    let mut next = support::publication(active);
    next.sequence += support::PUBLICATION_STEP;
    let mut publishing = Box::pin(fixture.backend.publish_connector(active, next));
    assert_pending(publishing.as_mut());
    fixture.assert_no_pending_connection();
    let response = support::unavailable(&request);
    let (observed, published) =
        bounded(async { tokio::join!(older, blocked.respond(&response)) }).await;
    published.unwrap();
    observed.unwrap();
    assert_eq!(
        fixture.without_request(publishing).await,
        Err(ConnectorError::StaleGeneration)
    );
}

#[tokio::test]
async fn cancelling_old_observation_releases_gate_without_replaying_it_or_blocking_registration() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let observer = fixture.backend.clone();
    let mut reading = Box::pin(observer.session(target().address));
    let (blocked, _) = accept_while(&fixture, &mut reading).await;
    let mut registration = Box::pin(
        fixture
            .backend
            .register_connector(uuid::Uuid::new_v4(), support::scope(target())),
    );
    assert_pending(registration.as_mut());
    fixture.assert_no_pending_connection();
    drop(reading);
    drop(blocked);
    let shell = view();
    let (enrolled, ()) = bounded(async {
        tokio::join!(
            registration,
            fixture.reply(|request| support::get(request, &shell))
        )
    })
    .await;
    let enrolled = enrolled.unwrap();
    let active = support::activate(&fixture, &enrolled, &shell).await;
    support::publish(&fixture, active, &shell).await;
}

#[tokio::test]
async fn cancelling_connector_admission_releases_gate_for_catalogue_and_keeps_previous_activation()
{
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let shell = view();
    let enrollment = support::register(&fixture, &shell).await;
    let active = support::activate(&fixture, &enrollment, &shell).await;
    let mut pending = Box::pin(fixture.backend.activate_connector(
        enrollment.connector_id,
        enrollment.credential,
        ConnectorInstanceId::new(),
    ));
    let (blocked, _) = accept_while(&fixture, &mut pending).await;
    let observer = fixture.backend.clone();
    let mut reading = Box::pin(observer.observe_agents());
    assert_pending(reading.as_mut());
    fixture.assert_no_pending_connection();
    drop(pending);
    drop(blocked);
    let (observed, ()) = bounded(async {
        tokio::join!(
            reading,
            fixture.reply(|request| support::list(request, vec![shell.clone()], true))
        )
    })
    .await;
    assert!(observed.unwrap().is_empty());
    support::publish(&fixture, active, &shell).await;
}
