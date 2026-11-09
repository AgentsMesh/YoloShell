use super::{
    attach_support::RunningAttach,
    federation_agents::ping,
    federation_support::{RunningNode, deadline},
};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_relay_control_protocol::StatusKind;

pub async fn exercise(node: &RunningNode, target: RuntimeTarget) {
    let mut helper = RunningAttach::through(
        &node.socket,
        node.node.hub_id(),
        target,
        uuid::Uuid::new_v4(),
        None,
    );
    helper.expect_status(StatusKind::ControlAcquired).await;
    let (command, marker) = ping();
    helper.input(&command).await;
    helper.marker(&marker).await;
    helper.stop(true).await;
    let mut client = node.client().await;
    let Reply::LeaseObservation { lease } = client
        .call(
            target.address.hub_id,
            Command::ObserveLease { target },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    assert!(lease.is_none());
}
