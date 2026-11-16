use crate::{Command, Reply};

/// Correlation for the agent-run query group, kept out of the main
/// response_target match so each handler stays within the source limit.
pub(crate) fn matches_target(reply: &Reply, command: &Command) -> Option<bool> {
    Some(match (reply, command) {
        (Reply::AgentRun { run }, Command::UpdateAgentRun { change }) => {
            run.address == change.fields().address
                && run.revision == change.fields().expected_revision.saturating_add(1)
        }
        (Reply::AgentRun { run }, Command::GetAgentRun { request }) => {
            match &request.fields().selector {
                yolo_hub_agent_run_protocol::AgentRunSelector::Address(address) => {
                    run.address == *address
                }
                yolo_hub_agent_run_protocol::AgentRunSelector::Name(name) => {
                    run.name.eq_ignore_ascii_case(name)
                        || run
                            .aliases
                            .iter()
                            .any(|alias| alias.eq_ignore_ascii_case(name))
                }
            }
        }
        (Reply::AgentRuns { list }, Command::ListAgentRuns { request }) => {
            let limit = request.fields().limit as usize;
            let after = request.fields().after;
            list.runs.len() <= limit
                && list
                    .runs
                    .windows(2)
                    .all(|pair| pair[0].address < pair[1].address)
                && after.is_none_or(|after| list.runs.iter().all(|run| run.address > after))
                && list
                    .next
                    .is_none_or(|next| list.runs.last().is_some_and(|run| run.address == next))
        }
        (Reply::AgentStop { stop }, Command::AgentStop { request }) => {
            match &request.fields().selector {
                yolo_hub_agent_run_protocol::AgentRunSelector::Address(address) => {
                    stop.run.address == *address
                }
                yolo_hub_agent_run_protocol::AgentRunSelector::Name(name) => {
                    stop.run.name.eq_ignore_ascii_case(name)
                        || stop
                            .run
                            .aliases
                            .iter()
                            .any(|alias| alias.eq_ignore_ascii_case(name))
                }
            }
        }
        _ => return None,
    })
}
