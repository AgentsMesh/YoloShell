use crate::fixtures::{constants::*, *};
use yolo_hub_rpc_protocol::{Command, Reply};

pub fn samples() -> Vec<(Command, Reply)> {
    vec![
        (
            Command::QueryCreation {
                request: yolo_hub_terminal_protocol::CreationQueryRequest::new(create()),
            },
            Reply::CreationQuery {
                query: yolo_hub_terminal_protocol::CreationQueryResult::new(
                    id(CREATION_ID),
                    yolo_hub_terminal_protocol::CreationQueryOutcome::Ready {
                        creation: creation(),
                    },
                )
                .unwrap(),
            },
        ),
        (
            Command::Create { request: create() },
            Reply::Created {
                creation: creation(),
            },
        ),
        (
            Command::TerminalRun {
                request: terminal_run(),
            },
            Reply::TerminalRun {
                run: yolo_hub_terminal_protocol::TerminalRunResult {
                    address: target().address,
                },
            },
        ),
        (
            Command::AgentStart {
                request: agent_start(),
            },
            Reply::AgentStart {
                start: yolo_hub_agent_protocol::AgentStartResult {
                    address: target().address,
                    provider_id: "codex".into(),
                    binding: yolo_hub_agent_protocol::AgentStartBinding::Awaiting,
                    run: None,
                },
            },
        ),
        (
            Command::Finalize {
                creation_token: id(CREATION_ID),
                creation: creation(),
            },
            Reply::Completed {},
        ),
    ]
}
