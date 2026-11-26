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
            request.fields().selector.matches(run)
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
            request.fields().selector.matches(&stop.run)
        }
        _ => return None,
    })
}
