use crate::{constants::*, record::Record};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeMap, path::PathBuf};
use yolo_agent_provider_contract::ProjectionPolicies;
use yolo_hub_agent_protocol::AgentSnapshot;
use yolo_hub_identity::{AgentAddress, AgentTarget, HubId, TerminalId};
use yolo_terminal_model::TerminalView;
use yolo_paths::{
    coordination::CoordinationLock, directory::PrivateDirectory, namespace_lease::NamespaceLease,
};

pub struct AgentProjection {
    pub(crate) record: Record,
    pub(crate) policies: ProjectionPolicies,
    path: PathBuf,
    namespace: NamespaceLease,
    lock: CoordinationLock,
}

impl AgentProjection {
    /// The directory must come from the retained Hub state, not from an RPC path.
    pub fn open(directory: PrivateDirectory, hub: HubId) -> Result<Self> {
        Self::open_with_policies(directory, hub, ProjectionPolicies::none())
    }

    pub fn open_with_policies(
        directory: PrivateDirectory,
        hub: HubId,
        policies: ProjectionPolicies,
    ) -> Result<Self> {
        let identity = directory.durable_identity()?;
        let root = directory.path().to_path_buf();
        let lock = CoordinationLock::try_acquire(&root.join(LOCK_FILE), MAX_LOCK_BYTES)?
            .context(ALREADY_OPEN)?;
        let namespace = NamespaceLease::from_directories(vec![(NAMESPACE, directory)])?;
        let path = root.join(RECORD_FILE);
        let record = if namespace.file_is_missing(&path)? {
            let record = Record {
                schema: SCHEMA_VERSION,
                hub,
                directory: identity,
                entries: BTreeMap::new(),
                latest: BTreeMap::new(),
                retired_controls: BTreeMap::new(),
            };
            namespace.write_json(&path, &record)?;
            record
        } else {
            let mut record: Record = namespace.read_json(&path)?;
            record.validate(hub, identity)?;
            if record.schema != SCHEMA_VERSION {
                record.migrate()?;
                namespace.write_json(&path, &record)?;
            }
            record
        };
        let mut projection = Self {
            record,
            policies,
            path,
            namespace,
            lock,
        };
        // Restarting a reader cannot turn a persisted observation into fresh liveness.
        projection.mark_unreachable()?;
        Ok(projection)
    }

    pub fn hub_id(&self) -> HubId {
        self.record.hub
    }

    pub fn get(&self, address: AgentAddress) -> Result<Option<AgentSnapshot>> {
        self.verify()?;
        ensure!(address.hub_id == self.record.hub, WRONG_ORIGIN);
        Ok(self
            .record
            .entries
            .get(&address.agent_id)
            .map(|entry| entry.snapshot.clone()))
    }

    pub fn list(&self) -> Result<Vec<AgentSnapshot>> {
        self.verify()?;
        Ok(self
            .record
            .entries
            .values()
            .map(|entry| entry.snapshot.clone())
            .collect())
    }

    /// The runtime terminal for an exact current execution target.
    pub fn terminal_for(&self, target: yolo_hub_identity::AgentTarget) -> Option<TerminalId> {
        let entry = self.record.entries.get(&target.address.agent_id)?;
        (entry.snapshot.cursor().target() == target)
            .then(|| entry.snapshot.fields().terminal.address.terminal_id)
    }

    /// Executions that have not ended, selected from each terminal's latest logical agent.
    /// Historical records remain available through `list` and `get`.
    pub fn list_active(&self) -> Result<Vec<AgentSnapshot>> {
        self.verify()?;
        Ok(self
            .record
            .latest
            .values()
            .filter_map(|id| self.record.entries.get(id))
            .filter(|entry| {
                !entry.source_exited
                    && matches!(
                        entry.snapshot.fields().lifecycle,
                        yolo_hub_agent_protocol::Lifecycle::Starting {}
                            | yolo_hub_agent_protocol::Lifecycle::Running {}
                            | yolo_hub_agent_protocol::Lifecycle::Unknown {}
                    )
                    && entry.snapshot.fields().terminal.runtime_epoch == entry.runtime
                    && entry.snapshot.fields().terminal.address.terminal_id == entry.subject.terminal_id
            })
            .map(|entry| entry.snapshot.clone())
            .collect())
    }

    /// Terminal-to-execution joins for reads that expose a run's current
    /// execution. Same selection as [`Self::list_active`], without snapshots.
    pub fn active_targets(&self) -> Result<Vec<(TerminalId, AgentTarget)>> {
        Ok(self
            .list_active()?
            .into_iter()
            .map(|agent| {
                (agent.fields().terminal.address.terminal_id, agent.cursor().target())
            })
            .collect())
    }

    pub fn mark_unreachable(&mut self) -> Result<()> {
        let mut next = self.record.clone();
        next.invalidate(None)?;
        self.commit(next)
    }

    /// Mark one terminal's latest execution unreachable. A failed scoped read
    /// proves nothing about sessions it never asked the Host for.
    pub fn mark_session_unreachable(&mut self, session: TerminalId) -> Result<()> {
        let mut next = self.record.clone();
        next.invalidate(Some(session))?;
        self.commit(next)
    }

    pub fn mark_missing_sessions(
        &mut self,
        observed: &std::collections::BTreeSet<yolo_hub_identity::TerminalId>,
    ) -> Result<()> {
        let missing = self
            .record
            .latest
            .keys()
            .filter(|id| !observed.contains(id))
            .copied()
            .collect::<Vec<_>>();
        let mut next = self.record.clone();
        for session in missing {
            next.invalidate(Some(session))?;
        }
        self.commit(next)
    }

    pub fn verify(&self) -> Result<()> {
        self.lock.verify()?;
        self.namespace.verify()
    }

    pub(crate) fn commit(&mut self, next: Record) -> Result<()> {
        self.verify()?;
        next.validate(self.record.hub, self.record.directory)?;
        if next != self.record {
            self.namespace.write_json(&self.path, &next)?;
            self.verify()?;
            self.record = next;
        }
        Ok(())
    }
}
