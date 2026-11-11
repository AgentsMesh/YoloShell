//! Reply validity for agent observation and control, kept out of the main
//! method match so each handler stays within the source limit.

use crate::{Method, Reply};
use yolo_hub_identity::HubId;

pub(crate) fn valid(reply: &Reply, method: Method, origin: HubId) -> Option<bool> {
    Some(match (reply, method) {
        (Reply::AgentControl { receipt }, Method::AgentControl) => {
            receipt.target().address.hub_id == origin
        }
        (Reply::AgentPrompt { prompt }, Method::AgentPrompt) => {
            prompt.receipt.target().address.hub_id == origin && prompt.terminal.hub_id == origin
        }
        (Reply::AgentStart { start }, Method::AgentStart) => start.address.hub_id == origin,
        (Reply::AgentStop { stop }, Method::AgentStop) => stop.run.address.hub_id == origin,
        (Reply::Agent { agent }, Method::GetAgent | Method::GetCurrentAgent) => {
            agent.cursor().target().address.hub_id == origin
        }
        // The reply echoes every evaluated target so run selectors, which the
        // Hub expands, are bounded and correlated without trusting the reply.
        (Reply::AgentWait { wait }, Method::WaitAgents) => {
            !wait.targets.is_empty()
                && wait.targets.len() <= yolo_hub_agent_protocol::MAX_AGENT_WAIT_TARGETS
                && wait.targets.iter().enumerate().all(|(i, target)| {
                    target.address.hub_id == origin && !wait.targets[..i].contains(target)
                })
                && wait.matched.len() <= yolo_hub_agent_protocol::MAX_AGENT_WAIT_TARGETS
                && wait.ended.len() <= yolo_hub_agent_protocol::MAX_AGENT_WAIT_TARGETS
                && wait
                    .matched
                    .iter()
                    .all(|agent| agent.cursor().target().address.hub_id == origin)
        }
        _ => return None,
    })
}
