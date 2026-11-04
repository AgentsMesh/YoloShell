use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::PublicError;

use crate::AgentRunService;

/// Route the agent-run query group. The backend keeps one service handle.
pub async fn agent_run_dispatch(
    service: Option<&AgentRunService>,
    command: &Command,
) -> Result<Reply, PublicError> {
    let service = service.ok_or(PublicError::Unsupported)?;
    match command {
        Command::GetAgentRun { request } => Ok(Reply::AgentRun {
            run: service.get(request).await?,
        }),
        Command::ListAgentRuns { request } => Ok(Reply::AgentRuns {
            list: service.list(request).await?,
        }),
        Command::UpdateAgentRun { change } => Ok(Reply::AgentRun {
            run: service.update(change).await?,
        }),
        _ => Err(PublicError::Unsupported),
    }
}
