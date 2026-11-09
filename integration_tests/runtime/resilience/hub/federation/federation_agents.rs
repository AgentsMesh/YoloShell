use super::{
    federation_support::*,
    support::{acquire, input},
};
use crate::Harness;
use tokio::time::timeout_at;
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::*;

pub struct CreatedAgent {
    pub target: RuntimeTarget,
    pub request: CreateShellRequest,
    pub creation: CreateShellResult,
}

pub async fn launch(node: &RunningNode, harness: &Harness, name: &str) -> RuntimeTarget {
    launch_client(node.client().await, node.node.hub_id(), harness, name).await
}
pub async fn launch_queryable(node: &RunningNode, harness: &Harness, name: &str) -> CreatedAgent {
    launch_client_queryable(node.client().await, node.node.hub_id(), harness, name).await
}
pub async fn launch_managed(
    node: &super::management_support::ManagedNode,
    harness: &Harness,
    name: &str,
) -> RuntimeTarget {
    launch_client(node.client().await, node.hub, harness, name).await
}
pub(super) async fn launch_client(
    client: yolo_hub_client::LocalClient,
    hub: yolo_hub_identity::HubId,
    harness: &Harness,
    name: &str,
) -> RuntimeTarget {
    launch_client_queryable(client, hub, harness, name)
        .await
        .target
}
async fn launch_client_queryable(
    mut client: yolo_hub_client::LocalClient,
    hub: yolo_hub_identity::HubId,
    harness: &Harness,
    name: &str,
) -> CreatedAgent {
    let token = uuid::Uuid::new_v4();
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: token,
        name: Some(name.into()),
        cwd: harness.home.path().to_string_lossy().into_owned(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let Reply::Created { creation } = client
        .call(
            hub,
            Command::Create {
                request: request.clone(),
            },
            deadline(),
        )
        .await
        .unwrap_or_else(|error| {
            panic!(
                "{name}: {error}; host stderr: {}",
                harness.host_stderr_tail()
            )
        })
    else {
        panic!()
    };
    let view = harness
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    harness.track_view(&view);
    client
        .call(
            hub,
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
            hub,
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
        .call(hub, acquire(target, LeaseMode::Acquire {}), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    client
        .call(
            hub,
            input(
                grant,
                &crate::agent_recovery::federation_agent_command(harness.home.path()),
            ),
            deadline(),
        )
        .await
        .unwrap();
    client
        .call(hub, Command::Release { grant }, deadline())
        .await
        .unwrap();
    timeout_at(deadline(), async {
        loop {
            let view = harness
                .client()
                .get_session(target.address.terminal_id.as_uuid())
                .await
                .unwrap();
            if view.agent.as_ref().is_some_and(|agent| {
                agent.recovery_availability
                    == yolo_runtime::agent::AgentRecoveryAvailability::Available
            }) {
                break;
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await
    .unwrap();
    CreatedAgent {
        target,
        request,
        creation,
    }
}

pub fn ping() -> (Vec<u8>, String) {
    const LEFT: &str = "YOLO_FEDERATED_";
    const RIGHT: &str = "RESPONSE_";
    let nonce = uuid::Uuid::new_v4();
    let command = format!(
        "{} {LEFT} {RIGHT} {nonce}\r",
        crate::fake_agent_constants::PACKAGED_TUI_PING_COMMAND
    )
    .into_bytes();
    (command, format!("{LEFT}{RIGHT}{nonce}"))
}
