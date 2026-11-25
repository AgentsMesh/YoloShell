use super::{
    cleanup_rpc::{ProtocolFixture, bounded},
    connectors as support,
    fixtures::{agent_control::host_view, view},
};
use yolo_hub_agent_protocol::{Freshness, ObservationSource, Reachability};
use yolo_hub_connector_protocol::{ConnectorError, EnrollmentStatus};
use yolo_hub_identity::ConnectorInstanceId;

#[tokio::test]
async fn native_takeover_permanently_supersedes_connector_without_control_or_process_calls() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let shell = view();
    let enrollment = support::register(&fixture, &shell).await;
    let activation = support::activate(&fixture, &enrollment, &shell).await;
    support::publish(&fixture, activation, &shell).await;
    let native = host_view();
    let mut publication = support::publication(activation);
    publication.sequence += support::PUBLICATION_STEP;
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture.backend.publish_connector(activation, publication),
            fixture.reply(|request| support::get(request, &native))
        )
    })
    .await;
    assert_eq!(result, Err(ConnectorError::Conflict));
    let list = fixture
        .without_request(fixture.backend.list_connectors())
        .await
        .unwrap();
    assert_eq!(list.first().unwrap().status, EnrollmentStatus::Superseded);
    let agents = support::observe(&fixture, vec![native.clone()]).await;
    let current = agents
        .iter()
        .find(|agent| agent.fields().source == ObservationSource::ProviderHook)
        .unwrap();
    let old = agents
        .iter()
        .find(|agent| agent.fields().source == ObservationSource::Connector)
        .unwrap();
    assert_eq!(old.cursor().target(), activation.agent);
    assert_eq!(old.fields().freshness, Freshness::Stale);
    assert_eq!(old.fields().reachability, Reachability::Unreachable);
    assert_eq!(
        support::session(&fixture, &native).await.fields().agent,
        Some(current.cursor().target())
    );
    assert_eq!(
        fixture
            .without_request(fixture.backend.activate_connector(
                enrollment.connector_id,
                enrollment.credential,
                ConnectorInstanceId::new()
            ))
            .await,
        Err(ConnectorError::Unauthorized)
    );
}

#[tokio::test]
async fn host_failure_deactivates_generation_and_preserves_stale_observation_for_reactivation() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let shell = view();
    let enrollment = support::register(&fixture, &shell).await;
    let activation = support::activate(&fixture, &enrollment, &shell).await;
    support::publish(&fixture, activation, &shell).await;
    let mut next = support::publication(activation);
    next.sequence += support::PUBLICATION_STEP;
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture.backend.publish_connector(activation, next.clone()),
            fixture.reply(support::unavailable)
        )
    })
    .await;
    assert_eq!(result, Err(ConnectorError::Unavailable));
    assert_eq!(
        fixture
            .without_request(fixture.backend.publish_connector(activation, next))
            .await,
        Err(ConnectorError::StaleGeneration)
    );
    let (observed, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            fixture.reply(support::unavailable)
        )
    })
    .await;
    let observed = observed.unwrap();
    let [old] = observed.as_slice() else { panic!() };
    assert_eq!(old.cursor().target(), activation.agent);
    assert_eq!(old.fields().freshness, Freshness::Stale);
    assert_eq!(old.fields().reachability, Reachability::Unreachable);
    let renewed = support::activate(&fixture, &enrollment, &shell).await;
    assert_ne!(renewed.generation, activation.generation);
    support::publish(&fixture, renewed, &shell).await;
    assert_eq!(
        support::session(&fixture, &shell)
            .await
            .fields()
            .agent
            .map(|target| target.address),
        Some(enrollment.agent)
    );
}

#[tokio::test]
async fn changed_epoch_never_associates_the_old_connector_with_a_replacement_shell() {
    let fixture = ProtocolFixture::new().with_connectors();
    let shell = view();
    let enrollment = support::register(&fixture, &shell).await;
    let activation = support::activate(&fixture, &enrollment, &shell).await;
    support::publish(&fixture, activation, &shell).await;
    let mut replacement = shell.clone();
    replacement.runtime.as_mut().unwrap().runtime_epoch = uuid::Uuid::new_v4();
    let session = support::session(&fixture, &replacement).await;
    assert_eq!(session.fields().agent, None);
    assert_ne!(
        session.fields().runtime.unwrap().target,
        activation.terminal
    );
    assert_eq!(
        fixture
            .without_request(fixture.backend.list_connectors())
            .await
            .unwrap()
            .first()
            .unwrap()
            .status,
        EnrollmentStatus::Superseded
    );
    let history = support::observe(&fixture, vec![replacement]).await;
    let [old] = history.as_slice() else { panic!() };
    assert_eq!(old.fields().terminal, activation.terminal);
    assert_eq!(old.fields().freshness, Freshness::Stale);
}
