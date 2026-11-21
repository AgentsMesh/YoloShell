use crate::{Registry, state::State};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Reachability};
use yolo_hub_connector_protocol::{ConnectorError, EnrollmentStatus};
use yolo_hub_connector_store::Seed;
use yolo_hub_identity::{AgentTarget, RuntimeTarget, TerminalId};

/// Catalogue associations are display metadata, never runtime or writer authority.
#[derive(Default)]
pub struct Catalogue {
    pub agents: Vec<AgentSnapshot>,
    pub primaries: BTreeMap<RuntimeTarget, AgentTarget>,
    /// Enabled, current and reachable executions. Retired or disconnected
    /// snapshots stay in `agents` for history but must not become `current`.
    pub enabled_targets: Vec<(TerminalId, AgentTarget)>,
}

impl Catalogue {
    pub fn active_targets(&self) -> Vec<(TerminalId, AgentTarget)> {
        self.enabled_targets.clone()
    }
}

impl Registry {
    pub fn catalogue(&self) -> Result<Catalogue, ConnectorError> {
        let entries = self.entries()?;
        catalogue(entries.iter().map(|(seed, state)| (seed, state)), false)
    }

    pub fn last_known_catalogue(&self) -> Catalogue {
        catalogue(self.cache.values().map(|(seed, state)| (seed, state)), true)
            .expect(crate::constants::INVALID_STATE)
    }

    /// Cached targets only select which Host facts to collect; final observations are verified.
    pub fn terminals(&self) -> Vec<RuntimeTarget> {
        self.cache
            .values()
            .map(|(seed, _)| seed.scope.terminal)
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect()
    }
}

fn catalogue<'a>(
    entries: impl Iterator<Item = (&'a Seed, &'a State)>,
    stale: bool,
) -> Result<Catalogue, ConnectorError> {
    let mut agents = Vec::new();
    let mut primaries = BTreeMap::new();
    let mut enabled_targets = Vec::new();
    for (seed, state) in entries {
        let Some(snapshot) = &state.snapshot else {
            continue;
        };
        let snapshot = if stale {
            let mut fields = snapshot.fields().clone();
            fields.freshness = Freshness::Stale;
            fields.reachability = Reachability::Unreachable;
            AgentSnapshot::new(fields).map_err(|_| ConnectorError::Internal)?
        } else {
            snapshot.clone()
        };
        if state.status == EnrollmentStatus::Enabled {
            if primaries
                .insert(seed.scope.terminal, snapshot.cursor().target())
                .is_some()
            {
                return Err(ConnectorError::Conflict);
            }
            if snapshot.fields().freshness == Freshness::Current
                && snapshot.fields().reachability == Reachability::Reachable
                && !matches!(
                    snapshot.fields().lifecycle,
                    yolo_hub_agent_protocol::Lifecycle::Exited { .. }
                )
            {
                enabled_targets.push((
                    seed.scope.terminal.address.terminal_id,
                    snapshot.cursor().target(),
                ));
            }
        }
        agents.push(snapshot);
    }
    agents.sort_by_key(|snapshot| snapshot.cursor().target());
    Ok(Catalogue {
        agents,
        primaries,
        enabled_targets,
    })
}
