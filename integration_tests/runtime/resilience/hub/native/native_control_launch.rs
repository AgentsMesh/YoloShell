use super::{
    super::{
        federation_support::{RunningNode, deadline},
        support::{acquire, input},
    },
    assets::{Assets, quote},
};
use crate::Harness;
use std::path::Path;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

pub async fn launch(
    node: &RunningNode,
    host: &Harness,
    assets: &Assets,
    marker: &Path,
    name: &str,
) -> RuntimeTarget {
    let mut client = node.client().await;
    let token = uuid::Uuid::new_v4();
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: token,
        name: Some(name.into()),
        cwd: assets.first.to_str().unwrap().into(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let Reply::Created { creation } = client
        .call(node.node.hub_id(), Command::Create { request }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let view = host
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    host.track_view(&view);
    client
        .call(
            node.node.hub_id(),
            Command::Finalize {
                creation_token: token,
                creation,
            },
            deadline(),
        )
        .await
        .unwrap();
    let Reply::Terminal { terminal: session } = client
        .call(
            node.node.hub_id(),
            Command::Get {
                address: creation.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let target = session.fields().runtime.unwrap().target;
    let Reply::Lease { grant } = client
        .call(
            target.address.hub_id,
            acquire(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!()
    };
    let command = format!(
        "export {}='{}' {}='{}'; '{}'\r",
        yolo_runtime::agent::claude::constants::DATA_HOME_ENV,
        quote(&assets.data),
        crate::fake_agent_constants::NATIVE_CONTROL_SIGNAL_MARKER_ENV,
        quote(marker),
        quote(&assets.binary)
    );
    client
        .call(
            target.address.hub_id,
            input(grant, command.as_bytes()),
            deadline(),
        )
        .await
        .unwrap();
    client
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
    target
}
