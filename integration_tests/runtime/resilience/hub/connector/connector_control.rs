use super::{
    super::{
        federation_support::{RunningNode, deadline},
        support::acquire,
    },
    constants::*,
    shell,
};
use std::path::Path;
use yolo_hub_agent_protocol::*;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::LeaseMode;

pub async fn unsupported(node: &RunningNode, observed: &AgentSnapshot, marker: &Path) {
    let target = observed.fields().terminal;
    let mut client = node.client().await;
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
    let canary = format!(
        "printf '%s' '{FORBIDDEN_EFFECT}' >> '{}'\r",
        marker.to_str().unwrap().replace('\'', SHELL_QUOTE_ESCAPE)
    );
    let actions = [
        AgentAction::Interrupt {},
        AgentAction::Respond {
            interaction: InteractionReference {
                target: observed.cursor().target(),
                id: uuid::Uuid::new_v4(),
            },
            response: InteractionResponse::Text { text: canary },
        },
    ];
    for action in actions {
        let request = AgentControlRequest::new(AgentControlFields {
            schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: uuid::Uuid::new_v4(),
            target: observed.cursor().target(),
            expected_cursor: observed.cursor(),
            action,
        })
        .unwrap();
        let Reply::AgentControl { receipt } = client
            .call(
                target.address.hub_id,
                Command::AgentControl {
                    request: request.clone(),
                    grant,
                },
                deadline(),
            )
            .await
            .unwrap()
        else {
            panic!()
        };
        assert_eq!(receipt.request_id(), request.fields().request_id);
        assert_eq!(receipt.target(), observed.cursor().target());
        assert_eq!(
            receipt.outcome(),
            CommandOutcome::Failed {
                code: FailureCode::Unsupported
            }
        );
    }
    client
        .call(
            target.address.hub_id,
            Command::Release { grant },
            deadline(),
        )
        .await
        .unwrap();
    shell::ping(node, target).await;
    assert_eq!(std::fs::read(marker).unwrap(), PRIVATE_CONTENT);
}
