use crate::fixtures::{constants::*, *};
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentPromptFields, AgentPromptOutcome, AgentPromptReference,
    AgentPromptRequest, AgentPromptResult, AgentPromptWait, CommandReceipt, Prompt,
};
use yolo_hub_rpc_protocol::{Command, Reply};

const PROMPT_TEXT: &str = "hello";

pub fn samples() -> Vec<(Command, Reply)> {
    let target = agent().cursor().target();
    let request = AgentPromptRequest::new(AgentPromptFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: id(REQUEST_ID),
        reference: AgentPromptReference::Execution { target },
        prompt: Prompt::new(PROMPT_TEXT.into()).unwrap(),
        wait: AgentPromptWait::None,
        take_control: false,
    })
    .unwrap();
    let receipt = CommandReceipt::accepted(id(REQUEST_ID), target).unwrap();
    vec![(
        Command::AgentPrompt { request },
        Reply::AgentPrompt {
            prompt: AgentPromptResult {
                receipt,
                terminal: address(HUB_ID, SESSION_ID),
                outcome: AgentPromptOutcome::Delivered,
                snapshot: None,
            },
        },
    )]
}
