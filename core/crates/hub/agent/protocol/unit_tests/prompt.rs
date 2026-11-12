use crate::fixtures::target;
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentPromptFields, AgentPromptReference, AgentPromptRequest,
    AgentPromptWait, MAX_PROMPT_WAIT_MS, Prompt,
};
use yolo_hub_identity::{HubId, TerminalAddress, TerminalId};

fn fields(reference: AgentPromptReference, wait: AgentPromptWait) -> AgentPromptFields {
    AgentPromptFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: Uuid::new_v4(),
        reference,
        prompt: Prompt::new("hello".into()).unwrap(),
        wait,
        take_control: false,
    }
}

#[test]
fn execution_and_terminal_references_validate() {
    assert!(
        AgentPromptRequest::new(fields(
            AgentPromptReference::Execution { target: target() },
            AgentPromptWait::None,
        ))
        .is_ok()
    );
    let address = TerminalAddress {
        hub_id: HubId::new(),
        terminal_id: TerminalId::new(),
    };
    assert!(
        AgentPromptRequest::new(fields(
            AgentPromptReference::Terminal { address },
            AgentPromptWait::TurnEnd {
                timeout_ms: MAX_PROMPT_WAIT_MS,
            },
        ))
        .is_ok()
    );
}

#[test]
fn wait_bounds_and_schema_are_enforced() {
    let reference = AgentPromptReference::Execution { target: target() };
    for timeout_ms in [0, MAX_PROMPT_WAIT_MS + 1] {
        assert!(
            AgentPromptRequest::new(fields(
                reference.clone(),
                AgentPromptWait::TurnEnd { timeout_ms },
            ))
            .is_err()
        );
    }
    let mut unsupported = fields(reference, AgentPromptWait::None);
    unsupported.schema_version = AGENT_PROTOCOL_SCHEMA_VERSION + 1;
    assert!(AgentPromptRequest::new(unsupported).is_err());
}
