use super::{
    super::federation_support::deadline, constants::*, node::ConnectorNode, process::Publisher,
};
use yolo_hub_agent_protocol::{Activity, AgentWorkingDirectory, Lifecycle, ObservedProvider};
use yolo_hub_connector_client::Client;
use yolo_hub_connector_protocol::*;
use yolo_hub_identity::{ConnectorInstanceId, RuntimeTarget};

pub async fn admin(node: &ConnectorNode, command: Command) -> Reply {
    let hub = node.running().node.hub_id();
    let mut client = Client::connect_local(&node.socket, hub, deadline())
        .await
        .unwrap();
    let result = client
        .call(Request::new(hub, command).unwrap(), deadline())
        .await;
    match result {
        Ok(reply) => reply,
        Err(yolo_hub_connector_client::ClientError::Remote(code)) => Reply::Error { code },
        Err(error) => panic!("Connector administration failed: {error:?}"),
    }
}
pub async fn enroll(node: &ConnectorNode, terminal: RuntimeTarget) -> Enrollment {
    let scope = ConnectorScope {
        terminal,
        provider: ObservedProvider::new(PROVIDER_KIND.into(), None, None).unwrap(),
    };
    let Reply::Enrolled { enrollment } = admin(node, Command::Register { scope }).await else {
        panic!()
    };
    enrollment
}
pub async fn activate_publisher(
    publisher: &mut Publisher,
    enrollment: &Enrollment,
    instance_id: ConnectorInstanceId,
) -> Activation {
    let Reply::Activated { activation } = publisher.call(activation(enrollment, instance_id)).await
    else {
        panic!()
    };
    assert_eq!(activation.agent.address, enrollment.agent);
    assert_eq!(activation.terminal, enrollment.scope.terminal);
    activation
}
pub fn activation(enrollment: &Enrollment, instance_id: ConnectorInstanceId) -> Command {
    Command::Activate {
        connector_id: enrollment.connector_id,
        credential: enrollment.credential,
        instance_id,
    }
}
pub fn publication(active: Activation, sequence: u64) -> Publication {
    Publication {
        connector_id: active.connector_id,
        generation: active.generation,
        agent: active.agent,
        terminal: active.terminal,
        sequence,
        state: ReportedState {
            lifecycle: Lifecycle::Running {},
            activity: Activity::Working,
            working_directory: Some(AgentWorkingDirectory::new(REPORTED_DIRECTORY.into()).unwrap()),
        },
    }
}
pub async fn publish(publisher: &mut Publisher, publication: Publication) -> PublicationAck {
    let Reply::Published { ack } = publisher
        .call(Command::Publish {
            publication: publication.clone(),
        })
        .await
    else {
        panic!()
    };
    assert_eq!(ack.connector_id, publication.connector_id);
    assert_eq!(ack.generation, publication.generation);
    assert_eq!(ack.sequence, publication.sequence);
    assert_eq!(ack.cursor.target(), publication.agent);
    ack
}
pub async fn status(node: &ConnectorNode, enrollment: &Enrollment) -> EnrollmentSummary {
    let Reply::Enrollments { enrollments } = admin(node, Command::List {}).await else {
        panic!()
    };
    enrollments
        .into_iter()
        .find(|item| item.connector_id == enrollment.connector_id)
        .unwrap()
}
