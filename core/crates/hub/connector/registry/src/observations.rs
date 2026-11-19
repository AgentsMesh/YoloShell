use crate::{Registry, RuntimeAdmission, constants::REVISION_STEP, state::State};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, ObservationCursor, Reachability};
use yolo_hub_connector_protocol::{Activation, ConnectorError, EnrollmentStatus};
use yolo_hub_connector_store::Seed;
use yolo_hub_identity::{ConnectorId, RuntimeTarget};

impl Registry {
    /// A partial Host read updates only the exact targets actually observed.
    pub fn reconcile_observed(
        &mut self,
        facts: &BTreeMap<RuntimeTarget, RuntimeAdmission>,
    ) -> Result<(), ConnectorError> {
        for (seed, state) in self.entries()? {
            if let Some(fact) = facts.get(&seed.scope.terminal) {
                self.reconcile_entry(seed, state, *fact)?;
            }
        }
        Ok(())
    }
    pub fn reconcile(
        &mut self,
        facts: &BTreeMap<RuntimeTarget, RuntimeAdmission>,
    ) -> Result<(), ConnectorError> {
        for (seed, state) in self.entries()? {
            let fact = facts
                .get(&seed.scope.terminal)
                .copied()
                .unwrap_or(RuntimeAdmission::Unavailable);
            self.reconcile_entry(seed, state, fact)?;
        }
        Ok(())
    }
    pub(crate) fn reconcile_one(
        &mut self,
        id: ConnectorId,
        admission: RuntimeAdmission,
    ) -> Result<(), ConnectorError> {
        let (seed, state) = self.entry(id)?;
        self.reconcile_entry(seed, state, admission)
    }
    fn reconcile_entry(
        &mut self,
        seed: Seed,
        mut state: State,
        admission: RuntimeAdmission,
    ) -> Result<(), ConnectorError> {
        if state.status != EnrollmentStatus::Enabled {
            return Ok(());
        }
        if admission.require(seed.scope.terminal).is_ok() {
            return Ok(());
        }
        if admission.supersedes(seed.scope.terminal) {
            state.revision = state
                .revision
                .checked_add(REVISION_STEP)
                .ok_or(ConnectorError::Capacity)?;
            state.status = EnrollmentStatus::Superseded;
        }
        let changed = stale(&mut state)? || state.status != EnrollmentStatus::Enabled;
        if changed {
            self.save(&seed, state)?;
        }
        self.live.remove(&seed.connector_id);
        Ok(())
    }
    pub fn disconnect(&mut self, activation: Activation) -> Result<(), ConnectorError> {
        if self.live.get(&activation.connector_id) != Some(&activation.generation) {
            return Ok(());
        }
        let (seed, mut state) = self.entry(activation.connector_id)?;
        if stale(&mut state)? {
            self.save(&seed, state)?;
        }
        self.live.remove(&activation.connector_id);
        Ok(())
    }
    pub fn disconnect_all(&mut self) -> Result<(), ConnectorError> {
        for (seed, mut state) in self.entries()? {
            if stale(&mut state)? {
                self.save(&seed, state)?;
            }
        }
        self.live.clear();
        Ok(())
    }
    pub fn snapshots(&self) -> Result<Vec<AgentSnapshot>, ConnectorError> {
        Ok(self
            .entries()?
            .into_iter()
            .filter_map(|(_, state)| state.snapshot)
            .collect())
    }
    /// Previously validated data only, explicitly stale; never usable as mutation authority.
    pub fn last_known(&self) -> Vec<AgentSnapshot> {
        self.cache
            .values()
            .filter_map(|(_, state)| state.snapshot.as_ref())
            .map(|snapshot| {
                let mut fields = snapshot.fields().clone();
                fields.freshness = Freshness::Stale;
                fields.reachability = Reachability::Unreachable;
                AgentSnapshot::new(fields).expect(crate::constants::INVALID_SNAPSHOT)
            })
            .collect()
    }
}
pub(crate) fn stale(state: &mut State) -> Result<bool, ConnectorError> {
    let Some(snapshot) = &state.snapshot else {
        return Ok(false);
    };
    let mut fields = snapshot.fields().clone();
    if fields.freshness == Freshness::Stale && fields.reachability == Reachability::Unreachable {
        return Ok(false);
    }
    fields.freshness = Freshness::Stale;
    fields.reachability = Reachability::Unreachable;
    fields.cursor = ObservationCursor::new(
        fields.cursor.target(),
        fields.cursor.stream(),
        fields
            .cursor
            .sequence()
            .checked_add(REVISION_STEP)
            .ok_or(ConnectorError::Sequence)?,
    )
    .map_err(|_| ConnectorError::Internal)?;
    state.snapshot = Some(AgentSnapshot::new(fields).map_err(|_| ConnectorError::Internal)?);
    Ok(true)
}
