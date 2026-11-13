use crate::{constants::*, types, memory};
use uuid::Uuid;
use yolo_hub_agent_protocol::{AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest, ObservationCursor, Prompt};
use yolo_hub_identity::*;
use yolo_hub_relay_control_protocol::{Request, RequestFields, SCHEMA_VERSION};

pub(super) unsafe fn decode(value: &types::Request) -> Result<Request, ()> {
    let hub = HubId::from_uuid(uuid(value.hub)).map_err(|_| ())?;
    let target = AgentTarget { address: AgentAddress { hub_id: hub,
        agent_id: AgentId::from_uuid(uuid(value.agent)).map_err(|_| ())? },
        execution_id: ExecutionId::from_uuid(uuid(value.execution)).map_err(|_| ())? };
    let prompt = unsafe { memory::text(value.prompt, yolo_hub_agent_protocol::MAX_PROMPT_BYTES)? };
    let action = match value.action {
        INTERRUPT if prompt.is_empty() => AgentAction::Interrupt {},
        SUBMIT => AgentAction::SubmitPrompt { prompt: Prompt::new(prompt.to_owned()).map_err(|_| ())? },
        _ => return Err(()),
    };
    Request::new(RequestFields { schema_version: SCHEMA_VERSION, launch_id: uuid(value.launch),
        terminal: RuntimeTarget { address: TerminalAddress { hub_id: hub,
            terminal_id: TerminalId::from_uuid(uuid(value.terminal)).map_err(|_| ())? },
            runtime_epoch: RuntimeEpoch::from_uuid(uuid(value.runtime)).map_err(|_| ())? },
        request: AgentControlRequest::new(AgentControlFields { schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
            request_id: uuid(value.request), target,
            expected_cursor: ObservationCursor::new(target, uuid(value.stream), value.sequence).map_err(|_| ())?, action,
        }).map_err(|_| ())?,
    }).map_err(|_| ())
}

pub(super) fn encode(value: &Request, output: &mut types::Decoded) {
    let fields = value.fields();
    let control = fields.request.fields();
    let (action, prompt) = match &control.action {
        AgentAction::Interrupt {} => (INTERRUPT, String::new()),
        AgentAction::SubmitPrompt { prompt } => (SUBMIT, prompt.text().to_owned()),
        AgentAction::Respond { .. } => unreachable!("validated relay actions are closed"),
    };
    output.prompt = memory::owned(prompt.into_bytes());
    output.request = types::Request {
        launch: identity(fields.launch_id), hub: identity(fields.terminal.address.hub_id.as_uuid()),
        terminal: identity(fields.terminal.address.terminal_id.as_uuid()), runtime: identity(fields.terminal.runtime_epoch.as_uuid()),
        request: identity(control.request_id), agent: identity(control.target.address.agent_id.as_uuid()),
        execution: identity(control.target.execution_id.as_uuid()), stream: identity(control.expected_cursor.stream()),
        sequence: control.expected_cursor.sequence(), action,
        prompt: types::Text { data: output.prompt.data, length: output.prompt.length },
    };
}

fn uuid(value: types::Identity) -> Uuid { Uuid::from_u64_pair(value.high, value.low) }
fn identity(value: Uuid) -> types::Identity { let (high, low) = value.as_u64_pair(); types::Identity { high, low } }
