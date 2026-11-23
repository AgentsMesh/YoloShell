use crate::{ProtocolError, Request, RequestFields, SCHEMA_VERSION};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    AGENT_PROTOCOL_SCHEMA_VERSION, AgentAction, AgentControlFields, AgentControlRequest,
    ObservationCursor, Prompt,
};
use yolo_hub_identity::*;
use yolo_native_relay_schema::relay_capnp::{control_request, identity};

pub(crate) fn encode(mut target: control_request::Builder<'_>, request: &Request) {
    let fields = request.fields();
    let control = fields.request.fields();
    put(target.reborrow().init_launch(), fields.launch_id);
    put(
        target.reborrow().init_hub(),
        fields.terminal.address.hub_id.as_uuid(),
    );
    put(
        target.reborrow().init_terminal(),
        fields.terminal.address.terminal_id.as_uuid(),
    );
    put(
        target.reborrow().init_runtime(),
        fields.terminal.runtime_epoch.as_uuid(),
    );
    put(target.reborrow().init_request(), control.request_id);
    put(
        target.reborrow().init_agent(),
        control.target.address.agent_id.as_uuid(),
    );
    put(
        target.reborrow().init_execution(),
        control.target.execution_id.as_uuid(),
    );
    put(
        target.reborrow().init_stream(),
        control.expected_cursor.stream(),
    );
    target.set_sequence(control.expected_cursor.sequence());
    match &control.action {
        AgentAction::Interrupt {} => target.init_action().set_interrupt(()),
        AgentAction::SubmitPrompt { prompt } => target.init_action().set_prompt(prompt.text()),
        AgentAction::Respond { .. } => {
            unreachable!("request validation rejects interaction responses")
        }
    }
}

pub(crate) fn decode(value: control_request::Reader<'_>) -> Result<Request, ProtocolError> {
    fn fail<E>(_: E) -> ProtocolError {
        ProtocolError::InvalidFrame
    }
    let hub = HubId::from_uuid(get(value.get_hub().map_err(fail)?)?).map_err(fail)?;
    let target = AgentTarget {
        address: AgentAddress {
            hub_id: hub,
            agent_id: AgentId::from_uuid(get(value.get_agent().map_err(fail)?)?).map_err(fail)?,
        },
        execution_id: ExecutionId::from_uuid(get(value.get_execution().map_err(fail)?)?)
            .map_err(fail)?,
    };
    let action = match value.get_action().which().map_err(fail)? {
        control_request::action::Interrupt(()) => AgentAction::Interrupt {},
        control_request::action::Prompt(text) => AgentAction::SubmitPrompt {
            prompt: Prompt::new(text.map_err(fail)?.to_str().map_err(fail)?.to_owned())
                .map_err(fail)?,
        },
    };
    let request = AgentControlRequest::new(AgentControlFields {
        schema_version: AGENT_PROTOCOL_SCHEMA_VERSION,
        request_id: get(value.get_request().map_err(fail)?)?,
        target,
        expected_cursor: ObservationCursor::new(
            target,
            get(value.get_stream().map_err(fail)?)?,
            value.get_sequence(),
        )
        .map_err(fail)?,
        action,
    })
    .map_err(fail)?;
    Request::new(RequestFields {
        schema_version: SCHEMA_VERSION,
        launch_id: get(value.get_launch().map_err(fail)?)?,
        terminal: RuntimeTarget {
            address: TerminalAddress {
                hub_id: hub,
                terminal_id: TerminalId::from_uuid(get(value.get_terminal().map_err(fail)?)?)
                    .map_err(fail)?,
            },
            runtime_epoch: RuntimeEpoch::from_uuid(get(value.get_runtime().map_err(fail)?)?)
                .map_err(fail)?,
        },
        request,
    })
}

fn put(mut target: identity::Builder<'_>, id: Uuid) {
    let (high, low) = id.as_u64_pair();
    target.set_high(high);
    target.set_low(low);
}
fn get(value: identity::Reader<'_>) -> Result<Uuid, ProtocolError> {
    let id = Uuid::from_u64_pair(value.get_high(), value.get_low());
    if id.is_nil() {
        Err(ProtocolError::InvalidIdentity)
    } else {
        Ok(id)
    }
}
