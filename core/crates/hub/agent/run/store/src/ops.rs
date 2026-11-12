//! Run identity operations: reserve, launch transition and queries.

use anyhow::Result;
use yolo_hub_agent_run_protocol::{
    AgentRunAddress, AgentRunSelector, AgentRunState, AgentRunSummary, GetAgentRunRequest,
    ListAgentRunsRequest, UpdateAgentRunRequest,
};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::PublicError;

use crate::StoreError;
use crate::record::{
    ReserveAgentRun, StoredRun, matches_filter, now_ms, stored_run, summary,
};
use crate::store::AgentRunStore;

impl AgentRunStore {
    pub fn reserve(&mut self, request: ReserveAgentRun) -> Result<AgentRunSummary, StoreError> {
        self.check()?;
        if let Some(existing) = self.record.find_token(request.start_token) {
            if existing.name.eq_ignore_ascii_case(&request.name)
                && existing.workspace == request.workspace
                && existing.parent == request.parent
            {
                if existing.state == AgentRunState::Starting {
                    self.active_starts.insert(existing.address);
                }
                return Ok(summary(existing));
            }
            return Err(PublicError::Conflict.into());
        }
        if self.record.find_name(&request.name).is_some() {
            return Err(PublicError::Conflict.into());
        }
        if let Some(parent) = request.parent {
            if parent.hub_id != self.record.hub_id {
                return Err(PublicError::OriginMismatch.into());
            }
            if self.record.find(parent).is_none() {
                return Err(PublicError::NotFound.into());
            }
        }
        crate::quota::admit(&self.record, &self.active_starts, request.parent)?;
        let run = stored_run(self.record.hub_id, request)?;
        self.commit(run.clone())?;
        self.active_starts.insert(run.address);
        Ok(summary(&run))
    }

    /// Return the existing run for this start token, if any. Retries must not
    /// create a second run or replay a launch for the same token.
    pub fn resume(&self, start_token: uuid::Uuid) -> Result<Option<AgentRunSummary>, StoreError> {
        self.check()?;
        Ok(self.record.find_token(start_token).map(summary))
    }

    pub fn mark_launched(
        &mut self,
        address: AgentRunAddress,
        terminal: TerminalAddress,
    ) -> Result<AgentRunSummary, StoreError> {
        self.check()?;
        let current = self
            .record
            .find(address)
            .cloned()
            .ok_or(PublicError::NotFound)?;
        if current.state == AgentRunState::Launched {
            if current.terminal == Some(terminal) {
                return Ok(summary(&current));
            }
            return Err(PublicError::Conflict.into());
        }
        let next = StoredRun {
            revision: current.revision + 1,
            state: AgentRunState::Launched,
            terminal: Some(terminal),
            updated_at_ms: now_ms(),
            ..current
        };
        self.commit(next.clone())?;
        self.active_starts.remove(&address);
        Ok(summary(&next))
    }

    pub fn get(&self, request: &GetAgentRunRequest) -> Result<Option<AgentRunSummary>, StoreError> {
        self.find_selector(&request.fields().selector)
    }

    pub fn find_selector(
        &self,
        selector: &AgentRunSelector,
    ) -> Result<Option<AgentRunSummary>, StoreError> {
        self.check()?;
        Ok(match selector {
            AgentRunSelector::Address(address) => self.record.find(*address).map(summary),
            AgentRunSelector::Name(name) => self.record.find_name(name).map(summary),
        })
    }

    /// Record a stop after the terminal effect was observed. Idempotent: an
    /// already-stopped run keeps its revision and timestamps.
    pub fn mark_stopped(
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
            return Ok(summary(&current));
        }
        if current.state == AgentRunState::Starting || current.terminal.is_none() {
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

    pub fn list(
        &self,
        request: &ListAgentRunsRequest,
    ) -> Result<(Vec<AgentRunSummary>, Option<AgentRunAddress>), StoreError> {
        self.check()?;
        let after = request.fields().after;
        let limit = request.fields().limit as usize;
        let mut rows: Vec<AgentRunSummary> = self
            .record
            .runs
            .iter()
            .filter(|run| after.is_none_or(|after| run.address > after))
            .filter(|run| matches_filter(run, request.fields().filter.as_ref()))
            .map(summary)
            .collect();
        rows.sort_by_key(|run| run.address);
        let more = rows.len() > limit;
        rows.truncate(limit);
        let next = more.then(|| rows.last().map(|row| row.address)).flatten();
        Ok((rows, next))
    }

    pub fn update(
        &mut self,
        request: &UpdateAgentRunRequest,
    ) -> Result<AgentRunSummary, StoreError> {
        self.check()?;
        let fields = request.fields();
        if fields.address.hub_id != self.record.hub_id {
            return Err(PublicError::OriginMismatch.into());
        }
        let current = self
            .record
            .find(fields.address)
            .cloned()
            .ok_or(PublicError::NotFound)?;
        if current.revision != fields.expected_revision {
            return Err(PublicError::Conflict.into());
        }
        let name = fields.name.clone().unwrap_or_else(|| current.name.clone());
        let aliases = fields
            .aliases
            .clone()
            .unwrap_or_else(|| current.aliases.clone());
        let labels = fields
            .labels
            .clone()
            .unwrap_or_else(|| current.labels.clone());
        for name in std::iter::once(&name).chain(aliases.iter()) {
            if let Some(other) = self.record.find_name(name) {
                if other.address != current.address {
                    return Err(PublicError::Conflict.into());
                }
            }
        }
        let next = StoredRun {
            revision: current.revision + 1,
            name,
            aliases,
            labels,
            updated_at_ms: now_ms(),
            ..current
        };
        self.commit(next.clone())?;
        Ok(summary(&next))
    }
}
