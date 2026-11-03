use yolo_hub_agent_protocol::AgentStartRequest;
use yolo_hub_terminal_protocol::{CreateShellRequest, TerminalRunRequest};
use yolo_terminal_model::{CreateTerminal, TerminalCallerContext};

/// Preserve the exact pre-resolution Host Create identity for both Create and Query.
/// Public request_id/schema_version are deliberately not part of this identity.
pub fn runtime_request(request: &CreateShellRequest) -> CreateTerminal {
    let fields = request.fields();
    CreateTerminal {
        creation_token: fields.creation_token,
        name: fields.name.clone(),
        cwd: fields.cwd.clone().into(),
        cols: Some(fields.size.cols()),
        rows: Some(fields.size.rows()),
        initial_input: None,
        context: None,
    }
}

/// A run request carries the same Host creation identity plus the bounded
/// launch input the Worker delivers after its hook ingress is ready.
pub fn runtime_run_request(
    request: &TerminalRunRequest,
    context: Option<TerminalCallerContext>,
) -> CreateTerminal {
    let fields = request.fields();
    CreateTerminal {
        creation_token: fields.creation_token,
        name: fields.name.clone(),
        cwd: fields.cwd.clone().into(),
        cols: Some(fields.size.cols()),
        rows: Some(fields.size.rows()),
        initial_input: Some(fields.input.clone()),
        context,
    }
}

/// A provider start carries the same Host creation identity plus the closed
/// launch executable selected by the Hub composition root.
pub fn runtime_agent_start_request(
    request: &AgentStartRequest,
    command: &str,
    context: Option<TerminalCallerContext>,
) -> CreateTerminal {
    let fields = request.fields();
    let mut input = command.as_bytes().to_vec();
    input.push(b'\r');
    CreateTerminal {
        creation_token: fields.start_token,
        name: fields.name.clone(),
        cwd: fields.cwd.clone().into(),
        cols: Some(fields.size.cols()),
        rows: Some(fields.size.rows()),
        initial_input: Some(input),
        context,
    }
}
