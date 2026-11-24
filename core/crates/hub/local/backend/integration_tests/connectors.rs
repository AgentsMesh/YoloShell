use super::{
    cleanup_rpc::{ProtocolFixture, bounded},
    connectors as support,
    fixtures::{self, view},
};
use yolo_hub_agent_protocol::{
    AgentCapabilities, CommandReceipt, FailureCode, Freshness, ObservationSource, Reachability,
    RecoveryStatus,
};
use yolo_hub_connector_protocol::{ConnectorCredential, ConnectorError, EnrollmentStatus};
use yolo_hub_identity::{ConnectorInstanceId, HubId};
use yolo_hub_rpc_protocol::{Command, Reply};

#[tokio::test]
async fn connector_observations_keep_shell_authority_separate_and_share_session_primary() {
    let fixture = ProtocolFixture::new().with_agents().with_connectors();
    let host = view();
    let enrollment = support::register(&fixture, &host).await;
    let activation = support::activate(&fixture, &enrollment, &host).await;
    let ack = support::publish(&fixture, activation, &host).await;
    assert!(!ack.duplicate);
    let observed = support::observe(&fixture, vec![host.clone()]).await;
    let [agent] = observed.as_slice() else {
        panic!()
    };
    assert_eq!(agent.cursor(), ack.cursor);
    assert_eq!(agent.fields().source, ObservationSource::Connector);
    assert_eq!(agent.fields().recovery, RecoveryStatus::Unsupported);
    assert_eq!(agent.fields().capabilities, AgentCapabilities::UNSUPPORTED);
    assert_eq!(
        agent.fields().working_directory.as_ref().unwrap().as_str(),
        support::DIRECTORY
    );
    assert_eq!(agent.fields().provider, enrollment.scope.provider);
    let session = support::session(&fixture, &host).await;
    assert_eq!(
        session.fields().agent,
        Some(agent.cursor().target())
    );
    assert_eq!(session.fields().cwd, host.manifest.cwd.to_str().unwrap());
    assert_eq!(
        session.fields().runtime.unwrap().target,
        activation.terminal
    );
    let request = fixtures::agent_control::request(agent);
    let command = Command::AgentControl {
        request: request.clone(),
        grant: fixtures::agent_control::authority(),
    };
    let (reply, ()) = bounded(async {
        tokio::join!(
            fixture.backend.execute(&command),
            fixture.reply(|request| support::get(request, &host))
        )
    })
    .await;
    assert_eq!(
        reply.unwrap(),
        Reply::AgentControl {
            receipt: CommandReceipt::rejected(&request, FailureCode::Unsupported)
        }
    );
    let revoked = fixture
        .without_request(fixture.backend.revoke_connector(
            uuid::Uuid::new_v4(),
            enrollment.connector_id,
            enrollment.revision,
        ))
        .await
        .unwrap();
    assert_eq!(revoked.status, EnrollmentStatus::Revoked);
    assert_eq!(support::session(&fixture, &host).await.fields().agent, None);
    let historical = support::observe(&fixture, vec![host]).await;
    assert_eq!(historical.len(), observed.len());
    assert_eq!(
        historical.first().unwrap().cursor().target(),
        agent.cursor().target()
    );
    assert_eq!(
        historical.first().unwrap().fields().freshness,
        Freshness::Stale
    );
    assert_eq!(
        historical.first().unwrap().fields().reachability,
        Reachability::Unreachable
    );
}

#[tokio::test]
async fn credentials_and_publication_targets_are_checked_before_any_host_read() {
    let fixture = ProtocolFixture::new().with_connectors();
    let host = view();
    let enrollment = support::register(&fixture, &host).await;
    let error = fixture
        .without_request(fixture.backend.activate_connector(
            enrollment.connector_id,
            ConnectorCredential::generate(),
            ConnectorInstanceId::new(),
        ))
        .await
        .unwrap_err();
    assert_eq!(error, ConnectorError::Unauthorized);
    let activation = support::activate(&fixture, &enrollment, &host).await;
    let mut invalid = support::publication(activation);
    invalid.generation = yolo_hub_identity::ConnectorGeneration::new();
    assert_eq!(
        fixture
            .without_request(fixture.backend.publish_connector(activation, invalid))
            .await,
        Err(ConnectorError::StaleGeneration)
    );
    let mut wrong_origin = support::scope(fixtures::target());
    wrong_origin.terminal.address.hub_id = HubId::new();
    assert_eq!(
        fixture
            .without_request(
                fixture
                    .backend
                    .register_connector(uuid::Uuid::new_v4(), wrong_origin)
            )
            .await
            .unwrap_err(),
        ConnectorError::InvalidRequest
    );
    assert_eq!(
        fixture
            .without_request(
                fixture
                    .backend
                    .register_connector(uuid::Uuid::nil(), support::scope(fixtures::target()))
            )
            .await
            .unwrap_err(),
        ConnectorError::InvalidRequest
    );
    fixture
        .without_request(fixture.backend.disconnect_connector(activation))
        .await
        .unwrap();
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
    fixture
        .without_request(fixture.backend.disconnect_all_connectors())
        .await
        .unwrap();
}
