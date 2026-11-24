use yolo_hub_agent_run_protocol::{AgentRunAddress, AgentRunState, AgentRunSummary};
use yolo_hub_terminal_protocol::PublicError;

use crate::StoreError;
use crate::record::{StoredRun, now_ms, summary};
use crate::store::AgentRunStore;

impl AgentRunStore {
    /// A Starting run with a recorded terminal reached the launch boundary
    /// before the previous process stopped; promote it. Terminal-less Starting
    /// runs stay resumable and are not counted by the new process until retried.
    pub(crate) fn reconcile_starting(&mut self) -> Result<(), StoreError> {
        let now = now_ms();
        let mut record = self.record.clone();
        let mut changed = false;
        for run in &mut record.runs {
            if run.state == AgentRunState::Starting && run.terminal.is_some() {
                run.state = AgentRunState::Launched;
                run.revision += 1;
                run.updated_at_ms = now;
                changed = true;
            }
        }
        if changed {
            self.commit_record(record)?;
        }
        Ok(())
    }

    /// A retried `Starting` run consumes a quota slot again.
    pub fn rejoin_starting(&mut self, address: AgentRunAddress) -> Result<(), StoreError> {
        self.check()?;
        if self.record.find(address).is_some_and(|run| run.state == AgentRunState::Starting) {
            self.active_starts.insert(address);
        }
        Ok(())
    }

    /// Compensates a reservation whose launch never created a terminal.
    pub fn cancel_starting(
        &mut self,
        address: AgentRunAddress,
    ) -> Result<AgentRunSummary, StoreError> {
        self.check()?;
        let current = self
            .record
            .find(address)
            .cloned()
            .ok_or(PublicError::NotFound)?;
        if current.state == AgentRunState::Stopped {
            self.active_starts.remove(&address);
            return Ok(summary(&current));
        }
        if current.state != AgentRunState::Starting {
            return Err(PublicError::Conflict.into());
        }
        let next = StoredRun {
            revision: current.revision + 1,
            state: AgentRunState::Stopped,
            updated_at_ms: now_ms(),
            ..current
        };
        self.commit(next.clone())?;
        self.active_starts.remove(&address);
        Ok(summary(&next))
    }
}
