use super::{constants::*, node::ConnectorNode, observe, process::Publisher, requests};
use yolo_hub_agent_protocol::{Activity, Freshness, Lifecycle};
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::ConnectorInstanceId;

pub async fn replace(
    node: &ConnectorNode,
    enrollment: &Enrollment,
    instance: ConnectorInstanceId,
    mut old: Publisher,
    previous: Activation,
) -> (Publisher, Activation) {
    let mut current = Publisher::start(&node.socket, node.running().node.hub_id()).await;
    let active = requests::activate_publisher(&mut current, enrollment, instance).await;
    assert_eq!(active.agent, previous.agent);
    assert_ne!(active.generation, previous.generation);
    requests::publish(
        &mut current,
        requests::publication(active, active.next_sequence),
    )
    .await;
    let rejected = old
        .call(Command::Publish {
            publication: requests::publication(previous, NEXT_SEQUENCE),
        })
        .await;
    assert_eq!(
        rejected,
        Reply::Error {
            code: ConnectorError::StaleGeneration
        }
    );
    // The old EOF arrives after replacement activation and cannot retire that generation.
    old.stop().await;
    requests::publish(&mut current, requests::publication(active, NEXT_SEQUENCE)).await;
    let observed = observe::agent(node.running(), enrollment.agent, Freshness::Current).await;
    assert_eq!(observed.cursor().target(), active.agent);
    (current, active)
}

pub async fn revoke(
    node: &ConnectorNode,
    enrollment: &Enrollment,
    publisher: &mut Publisher,
    active: Activation,
) {
    let exit_sequence = NEXT_SEQUENCE + FIRST_PUBLICATION_SEQUENCE;
    let mut exited = requests::publication(active, exit_sequence);
    exited.state.lifecycle = Lifecycle::Exited {
        exit_code: Some(EXIT_CODE),
    };
    exited.state.activity = Activity::Idle;
    requests::publish(publisher, exited).await;
    let running_shell = observe::session(node.running(), active.terminal).await;
    assert_eq!(running_shell.fields().agent, Some(active.agent));
    // Construct work under the old grant, then deliver it only after revocation commits.
    let queued = requests::publication(active, exit_sequence + FIRST_PUBLICATION_SEQUENCE);
    let revision_conflict = requests::admin(
        node,
        Command::Revoke {
            connector_id: enrollment.connector_id,
            expected_revision: enrollment.revision + FIRST_CONNECTOR_REVISION,
        },
    )
    .await;
    assert_eq!(
        revision_conflict,
        Reply::Error {
            code: ConnectorError::Conflict
        }
    );
    let Reply::Revoked {
        enrollment: revoked,
    } = requests::admin(
        node,
        Command::Revoke {
            connector_id: enrollment.connector_id,
            expected_revision: enrollment.revision,
        },
    )
    .await
    else {
        panic!()
    };
    assert_eq!(revoked.status, EnrollmentStatus::Revoked);
    assert_eq!(
        publisher
            .call(Command::Publish {
                publication: queued
            })
            .await,
        Reply::Error {
            code: ConnectorError::StaleGeneration
        }
    );
    let mut denied = Publisher::start(&node.socket, node.running().node.hub_id()).await;
    assert_eq!(
        denied
            .call(requests::activation(enrollment, ConnectorInstanceId::new()))
            .await,
        Reply::Error {
            code: ConnectorError::Unauthorized
        }
    );
    denied.stop().await;
    let current = observe::session(node.running(), active.terminal).await;
    assert!(current.fields().agent.is_none());
    assert_eq!(
        current.fields().runtime.unwrap().safe_termination,
        running_shell.fields().runtime.unwrap().safe_termination
    );
}
