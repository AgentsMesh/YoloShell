use crate::{
    AgentRunStore, StoreError,
    record::{StoredRun, now_ms, summary},
};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;
use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunState, AgentRunSummary};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::PublicError;

/// The Host's evidence about a terminal and, when present, its PTY incarnation.
#[derive(Clone, Copy)]
pub enum TerminalObservation {
    Live(Uuid),
    Ended(Uuid),
    Archived,
}

impl AgentRunStore {
    pub fn launched_terminals(&self) -> Result<BTreeSet<TerminalAddress>, StoreError> {
        self.check()?;
        Ok(self
            .record
            .runs
            .iter()
            .filter(|run| run.state == AgentRunState::Launched)
            .filter_map(|run| run.terminal)
            .collect())
    }

    pub fn observe_terminals(
        &mut self,
        observations: &BTreeMap<TerminalAddress, TerminalObservation>,
    ) -> Result<(), StoreError> {
        let changes = self
            .record
            .runs
            .iter()
            .enumerate()
            .filter_map(|(index, run)| {
                let observation = observations.get(&run.terminal?)?;
                observed_transition(run, *observation).map(|change| (index, change))
            })
            .collect::<Vec<_>>();
        if changes.is_empty() {
            return Ok(());
        }
        self.check()?;
        let mut record = self.record.clone();
        let now = now_ms();
        for (index, (state, ended, observed, archived)) in changes {
            transition(
                &mut record.runs[index],
                state,
                ended,
                observed,
                archived,
                now,
            )?;
        }
        self.commit_record(record)
    }

    /// Record an explicit stop after the Host confirms the effect.
    pub fn mark_stopped(
        &mut self,
        address: AgentRunAddress,
    ) -> Result<AgentRunSummary, StoreError> {
        self.mark_stopped_if_current(address, None, None, None)
    }

    pub fn mark_stopped_if_current(
        &mut self,
        address: AgentRunAddress,
        terminal: Option<TerminalAddress>,
        expected_runtime: Option<Uuid>,
        expected_observed_runtime: Option<Option<Uuid>>,
    ) -> Result<AgentRunSummary, StoreError> {
        self.check()?;
        let mut run = self
            .record
            .find(address)
            .cloned()
            .ok_or(PublicError::NotFound)?;
        if terminal.is_some() && run.terminal != terminal {
            return Err(PublicError::Conflict.into());
        }
        if let Some(expected) = expected_observed_runtime {
            if run.observed_runtime != expected {
                return Err(PublicError::Conflict.into());
            }
        }
        if run.state == AgentRunState::Stopped {
            if run.ended_runtime.is_none() && expected_runtime.is_none() {
                return Ok(summary(&run));
            }
            if run.ended_runtime == expected_runtime
                && expected_runtime.is_some()
                && expected_observed_runtime.is_none_or(|expected| run.observed_runtime == expected)
            {
                return Ok(summary(&run));
            }
            return Err(PublicError::Conflict.into());
        }
        if run.state == AgentRunState::Starting || run.terminal.is_none() {
            return Err(PublicError::Conflict.into());
        }
        let ended = expected_runtime.or(run.ended_runtime);
        let observed = expected_observed_runtime
            .flatten()
            .or(expected_runtime)
            .or(run.observed_runtime);
        let archived = run.terminal_archived;
        transition(
            &mut run,
            AgentRunState::Stopped,
            ended,
            observed,
            archived,
            now_ms(),
        )?;
        self.commit(run.clone())?;
        self.active_starts.remove(&address);
        Ok(summary(&run))
    }
}

fn observed_transition(
    run: &StoredRun,
    observation: TerminalObservation,
) -> Option<(AgentRunState, Option<Uuid>, Option<Uuid>, bool)> {
    use AgentRunState::{Launched, Stopped};
    use TerminalObservation::{Archived, Ended, Live};
    if run.terminal_archived {
        return None;
    }
    match (run.state, run.observed_runtime, observation) {
        (Launched, None, Live(epoch)) => Some((Launched, None, Some(epoch), false)),
        (AgentRunState::Starting, _, Live(epoch)) => {
            Some((AgentRunState::Starting, None, Some(epoch), false))
        }
        (Launched, observed, Live(epoch)) if observed == Some(epoch) => None,
        (Launched, Some(_), Live(epoch)) => Some((Launched, None, Some(epoch), false)),
        (Launched, Some(observed), Ended(epoch)) if observed == epoch => {
            Some((Stopped, Some(epoch), Some(epoch), false))
        }
        (Launched, None, Ended(epoch)) => Some((Stopped, Some(epoch), Some(epoch), false)),
        (Stopped, observed, Ended(epoch))
            if run.ended_runtime == Some(epoch) && observed == Some(epoch) =>
        {
            None
        }
        (Stopped, _, Ended(_)) => None,
        (Stopped, Some(ended), Live(epoch))
            if run.ended_runtime == Some(ended) && ended != epoch =>
        {
            Some((Launched, None, Some(epoch), false))
        }
        (Launched, observed, Archived) => Some((Stopped, observed, observed, true)),
        (Stopped, _, Archived) => Some((Stopped, run.ended_runtime, run.observed_runtime, true)),
        _ => None,
    }
}

fn transition(
    run: &mut StoredRun,
    state: AgentRunState,
    ended_runtime: Option<Uuid>,
    observed_runtime: Option<Uuid>,
    terminal_archived: bool,
    now: i64,
) -> Result<(), StoreError> {
    run.state = state;
    run.ended_runtime = ended_runtime;
    run.observed_runtime = observed_runtime;
    run.terminal_archived = terminal_archived;
    run.revision = run
        .revision
        .checked_add(1)
        .ok_or(PublicError::LimitExceeded)?;
    run.updated_at_ms = now.max(run.updated_at_ms);
    Ok(())
}
