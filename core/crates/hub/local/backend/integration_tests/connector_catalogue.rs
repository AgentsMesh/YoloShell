use super::{
    cleanup_rpc::{ProtocolFixture, bounded},
    connectors as support,
    fixtures::{self, agent_control::host_view, view},
};
use yolo_hub_agent_protocol::{Freshness, Reachability};
use yolo_hub_connector_protocol::{ConnectorError, EnrollmentStatus};
use yolo_hub_identity::{RuntimeEpoch, TerminalId};

const TWO_CONNECTORS: usize = 2;

#[tokio::test]
async fn querying_one_session_does_not_deactivate_another_connector_generation() {
    let fixture = ProtocolFixture::new().with_connectors();
    let first = view();
    let first_enrollment = support::register(&fixture, &first).await;
    let first_activation = support::activate(&fixture, &first_enrollment, &first).await;
    support::publish(&fixture, first_activation, &first).await;
    let mut second_target = fixtures::target();
    second_target.address.terminal_id = TerminalId::new();
    second_target.runtime_epoch = RuntimeEpoch::new();
    let mut second = view();
    second.manifest.terminal_id = second_target.address.terminal_id.as_uuid();
    second.runtime.as_mut().unwrap().runtime_epoch = second_target.runtime_epoch.as_uuid();
    let (enrollment, ()) = bounded(async {
        tokio::join!(
            fixture
                .backend
                .register_connector(uuid::Uuid::new_v4(), support::scope(second_target)),
            fixture.reply(|request| support::get(request, &second))
        )
    })
    .await;
    let enrollment = enrollment.unwrap();
    let activation = support::activate(&fixture, &enrollment, &second).await;
    support::publish(&fixture, activation, &second).await;
    assert_eq!(
        support::session(&fixture, &first)
            .await
            .fields()
            .agent
            .map(|target| target.address),
        Some(first_enrollment.agent)
    );
    let mut next = support::publication(activation);
    next.sequence += support::PUBLICATION_STEP;
    let (result, ()) = bounded(async {
        tokio::join!(
            fixture.backend.publish_connector(activation, next),
            fixture.reply(|request| support::get(request, &second))
        )
    })
    .await;
    assert!(!result.unwrap().duplicate);
    let observed = support::observe(&fixture, vec![first.clone(), second.clone()]).await;
    assert_eq!(observed.len(), TWO_CONNECTORS);
    assert!(
        observed
            .iter()
            .all(|agent| agent.fields().freshness == Freshness::Current)
    );
    let (terminals, ()) = bounded(async {
        tokio::join!(
            fixture.backend.terminals(false),
            fixture.reply(|request| support::list(request, vec![first, second], true))
        )
    })
    .await;
    let terminals = terminals.unwrap();
    assert_eq!(terminals.len(), TWO_CONNECTORS);
    for session in terminals {
        let agent = session.fields().agent.unwrap();
        assert!(
            observed
                .iter()
                .any(|observed| observed.cursor().target() == agent
                    && observed.fields().terminal == session.fields().runtime.unwrap().target)
        );
    }
}

#[tokio::test]
async fn incomplete_host_catalogue_deactivates_but_never_proves_native_supersession() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let shell = view();
    let enrollment = support::register(&fixture, &shell).await;
    let activation = support::activate(&fixture, &enrollment, &shell).await;
    support::publish(&fixture, activation, &shell).await;
    let (agents, ()) = bounded(async {
        tokio::join!(
            fixture.backend.observe_agents(),
            fixture.reply(|request| support::list(request, vec![host_view()], false))
        )
    })
    .await;
    let agents = agents.unwrap();
    let [agent] = agents.as_slice() else { panic!() };
    assert_eq!(agent.cursor().target(), activation.agent);
    assert_eq!(agent.fields().freshness, Freshness::Stale);
    assert_eq!(agent.fields().reachability, Reachability::Unreachable);
    assert_eq!(
        fixture
            .without_request(fixture.backend.list_connectors())
            .await
            .unwrap()
            .first()
            .unwrap()
            .status,
        EnrollmentStatus::Enabled
    );
    assert_eq!(
        fixture
            .without_request(
                fixture
                    .backend
                    .publish_connector(activation, support::publication(activation))
            )
            .await,
        Err(ConnectorError::StaleGeneration)
    );
    let restored = support::activate(&fixture, &enrollment, &shell).await;
    support::publish(&fixture, restored, &shell).await;
}

#[tokio::test]
async fn poisoned_registry_keeps_only_stale_history_without_a_primary_association() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let shell = view();
    let enrollment = support::register(&fixture, &shell).await;
    let activation = support::activate(&fixture, &enrollment, &shell).await;
    support::publish(&fixture, activation, &shell).await;
    fixture.invalidate_connector_storage();
    let agents = support::observe(&fixture, vec![shell.clone()]).await;
    let [agent] = agents.as_slice() else { panic!() };
    assert_eq!(agent.cursor().target(), activation.agent);
    assert_eq!(agent.fields().freshness, Freshness::Stale);
    assert_eq!(agent.fields().reachability, Reachability::Unreachable);
    assert_eq!(
        support::session(&fixture, &shell).await.fields().agent,
        None
    );
    assert_eq!(
        fixture
            .without_request(
                fixture
                    .backend
                    .publish_connector(activation, support::publication(activation))
            )
            .await,
        Err(ConnectorError::Unavailable)
    );
}
