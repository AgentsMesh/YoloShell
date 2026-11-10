use super::{
    cleanup_rpc::{ProtocolFixture, bounded},
    connector_barrier::{Observation, accept_while, assert_pending},
    connectors as support,
    fixtures::{target, view},
};
use yolo_hub_connector_protocol::EnrollmentStatus;
use yolo_hub_identity::ConnectorInstanceId;
use yolo_runtime::protocol::method;

#[tokio::test]
async fn older_catalogue_cannot_supersede_a_registration_admitted_for_a_new_runtime() {
    for observation in [
        Observation::Agents,
        Observation::Terminals,
        Observation::Terminal,
    ] {
        let fixture = ProtocolFixture::new().with_agents().with_connectors();
        let original = view();
        let mut replacement = original.clone();
        replacement.runtime.as_mut().unwrap().runtime_epoch = uuid::Uuid::new_v4();
        let mut next_target = target();
        next_target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::from_uuid(
            replacement.runtime.as_ref().unwrap().runtime_epoch,
        )
        .unwrap();
        let observing_backend = fixture.backend.clone();
        let mut older = Box::pin(observation.read(&observing_backend, target().address));
        let (mut blocked, request) = accept_while(&fixture, &mut older).await;
        // Host captured the old runtime, but its reply is delayed. A new admission
        // must not read a newer Host fact and commit ahead of this reconciliation.
        let mut registration = Box::pin(
            fixture
                .backend
                .register_connector(uuid::Uuid::new_v4(), support::scope(next_target)),
        );
        assert_pending(registration.as_mut());
        fixture.assert_no_pending_connection();
        let response = if request.method == method::GET_SESSION {
            support::get(&request, &original)
        } else {
            support::list(&request, vec![original], true)
        };
        let (observed, published) =
            bounded(async { tokio::join!(older, blocked.respond(&response)) }).await;
        published.unwrap();
        observed.unwrap();
        let (enrolled, ()) = bounded(async {
            tokio::join!(
                registration,
                fixture.reply(|request| support::get(request, &replacement))
            )
        })
        .await;
        let enrolled = enrolled.unwrap();
        assert_eq!(
            fixture.backend.list_connectors().await.unwrap()[0].status,
            EnrollmentStatus::Enabled
        );
        let active = support::activate(&fixture, &enrolled, &replacement).await;
        support::publish(&fixture, active, &replacement).await;
        let agents = support::observe(&fixture, vec![replacement]).await;
        assert_eq!(agents[0].cursor().target(), active.agent);
        assert_eq!(
            agents[0].fields().freshness,
            yolo_hub_agent_protocol::Freshness::Current
        );
        assert_eq!(
            fixture.backend.list_connectors().await.unwrap()[0].status,
            EnrollmentStatus::Enabled
        );
    }
}

#[tokio::test]
async fn older_failed_catalogue_cannot_deactivate_a_later_successful_reactivation() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let shell = view();
    let enrolled = support::register(&fixture, &shell).await;
    let old = support::activate(&fixture, &enrolled, &shell).await;
    support::publish(&fixture, old, &shell).await;
    let observing_backend = fixture.backend.clone();
    let mut older = Box::pin(observing_backend.observe_agents());
    let (mut blocked, request) = accept_while(&fixture, &mut older).await;
    let mut activation = Box::pin(fixture.backend.activate_connector(
        enrolled.connector_id,
        enrolled.credential,
        ConnectorInstanceId::new(),
    ));
    assert_pending(activation.as_mut());
    fixture.assert_no_pending_connection();
    let response = support::unavailable(&request);
    let (observed, published) =
        bounded(async { tokio::join!(older, blocked.respond(&response)) }).await;
    published.unwrap();
    assert_eq!(
        observed.unwrap()[0].fields().freshness,
        yolo_hub_agent_protocol::Freshness::Stale
    );
    let (active, ()) = bounded(async {
        tokio::join!(
            activation,
            fixture.reply(|request| support::get(request, &shell))
        )
    })
    .await;
    let active = active.unwrap();
    assert_ne!(active.generation, old.generation);
    support::publish(&fixture, active, &shell).await;
    let observed = support::observe(&fixture, vec![shell]).await;
    assert_eq!(
        observed[0].fields().freshness,
        yolo_hub_agent_protocol::Freshness::Current
    );
    assert_eq!(observed[0].cursor().target(), active.agent);
}
