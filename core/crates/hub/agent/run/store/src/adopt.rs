//! Adoption of provider executions that were observed before any durable run
//! existed, for example a provider started by hand inside a terminal.

use uuid::Uuid;
use yolo_hub_agent_run_protocol::{
    AgentRunState, AgentRunSummary, validate_provider_id,
};
use yolo_hub_identity::TerminalAddress;

use crate::StoreError;
use crate::record::{ReserveAgentRun, stored_run, summary};
use crate::store::AgentRunStore;

impl AgentRunStore {
    /// Materialize a launched run for one observed terminal execution.
    ///
    /// Adoption is idempotent per terminal and best effort: an execution that
    /// already has a run, an invalid provider, a taken name or a full active
    /// quota leaves the observation unadopted instead of failing the read that
    /// produced it.
    pub fn adopt(
        &mut self,
        terminal: TerminalAddress,
        provider_id: &str,
    ) -> Result<Option<AgentRunSummary>, StoreError> {
        self.check()?;
        if terminal.hub_id != self.record.hub_id
            || validate_provider_id(provider_id).is_err()
            || self.record.runs.iter().any(|run| run.terminal == Some(terminal))
            || crate::quota::full(&self.record, &self.active_starts)
        {
            return Ok(None);
        }
        let name = format!("observed-{}", terminal.terminal_id.as_uuid().simple());
        if self.record.find_name(&name).is_some() {
            return Ok(None);
        }
        let mut run = stored_run(
            self.record.hub_id,
            ReserveAgentRun {
                run_id: Uuid::new_v4(),
                name,
                provider_id: provider_id.to_string(),
                workspace: None,
                parent: None,
                start_token: Uuid::new_v4(),
            },
        )?;
        run.terminal = Some(terminal);
        run.state = AgentRunState::Launched;
        self.commit(run.clone())?;
        Ok(Some(summary(&run)))
    }
}
