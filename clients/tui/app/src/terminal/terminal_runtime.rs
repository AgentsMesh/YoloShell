use crate::{State, resize::PendingResize};
use std::{
    collections::{BTreeMap, BTreeSet},
    time::Instant,
};
use uuid::Uuid;
use yolo_hub_identity::{RuntimeEpoch, TerminalAddress};
use yolo_hub_terminal_protocol::TerminalSize;

/// Per-terminal runtime bookkeeping that is not the screen surface or the
/// writer grant. Facts are grouped by the lifecycle scope that clears them, so
/// adding a fact to a scope automatically enrolls it in that scope's cleanup.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct TerminalRuntimeFacts {
    offline: bool,
    ended: Option<RuntimeEpoch>,
    bound: BoundFacts,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct BoundFacts {
    writer_geometry: WriterGeometryFacts,
    connection: ConnectionFacts,
    observer_generation: Option<Uuid>,
    resync: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct WriterGeometryFacts {
    last_size: Option<TerminalSize>,
    geometry_syncing: bool,
    resize_pending: bool,
    resize_in_flight: Option<PendingResize>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct ConnectionFacts {
    renewal_pending: bool,
    codex_open_attempt: Option<Instant>,
}

impl TerminalRuntimeFacts {
    fn is_empty(self) -> bool {
        self == Self::default()
    }

    fn clear_writer_geometry_scope(&mut self) {
        self.bound.writer_geometry = Default::default();
    }

    fn clear_detached_binding(&mut self) {
        self.bound = Default::default();
    }

    fn clear_connection_scope(&mut self) {
        self.bound.writer_geometry = Default::default();
        self.bound.connection = Default::default();
    }
}

impl State {
    fn runtime_facts(&self, address: TerminalAddress) -> Option<&TerminalRuntimeFacts> {
        self.terminal_runtime.get(&address)
    }

    fn runtime_facts_mut(&mut self, address: TerminalAddress) -> &mut TerminalRuntimeFacts {
        self.terminal_runtime.entry(address).or_default()
    }

    fn prune_runtime_facts(&mut self, address: TerminalAddress) {
        if self
            .runtime_facts(address)
            .is_some_and(|facts| facts.is_empty())
        {
            self.terminal_runtime.remove(&address);
        }
    }

    pub(crate) fn is_offline(&self, address: TerminalAddress) -> bool {
        self.runtime_facts(address)
            .is_some_and(|facts| facts.offline)
    }

    pub(crate) fn mark_offline(&mut self, address: TerminalAddress) {
        self.runtime_facts_mut(address).offline = true;
    }

    pub(crate) fn clear_offline(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.offline = false;
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn ended_epoch(&self, address: TerminalAddress) -> Option<RuntimeEpoch> {
        self.runtime_facts(address).and_then(|facts| facts.ended)
    }

    pub(crate) fn ended_epoch_map(&self) -> BTreeMap<TerminalAddress, RuntimeEpoch> {
        self.terminal_runtime
            .iter()
            .filter_map(|(address, facts)| facts.ended.map(|epoch| (*address, epoch)))
            .collect()
    }

    pub(crate) fn mark_ended(&mut self, address: TerminalAddress, epoch: RuntimeEpoch) {
        self.runtime_facts_mut(address).ended = Some(epoch);
    }

    pub(crate) fn observer_generation(&self, address: TerminalAddress) -> Option<Uuid> {
        self.runtime_facts(address)
            .and_then(|facts| facts.bound.observer_generation)
    }

    pub(crate) fn set_observer_generation(&mut self, address: TerminalAddress, generation: Uuid) {
        self.runtime_facts_mut(address).bound.observer_generation = Some(generation);
    }

    pub(crate) fn clear_observer_generation(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.bound.observer_generation = None;
        }
        self.prune_runtime_facts(address);
    }

    /// Drops geometry facts for terminals that no longer hold a writer.
    pub(crate) fn retain_writer_runtime_facts(&mut self) {
        self.terminal_runtime.retain(|address, facts| {
            if self.writers.contains_key(address) {
                return true;
            }
            facts.clear_writer_geometry_scope();
            !facts.is_empty()
        });
    }

    pub(crate) fn clear_writer_geometry_facts(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.clear_writer_geometry_scope();
        }
        self.prune_runtime_facts(address);
    }

    pub(crate) fn clear_connection_runtime_facts(&mut self) {
        for facts in self.terminal_runtime.values_mut() {
            facts.clear_connection_scope();
        }
        self.terminal_runtime.retain(|_, facts| !facts.is_empty());
    }

    pub(crate) fn clear_detached_binding_facts(&mut self, address: TerminalAddress) {
        if let Some(facts) = self.terminal_runtime.get_mut(&address) {
            facts.clear_detached_binding();
        }
        self.prune_runtime_facts(address);
    }
}

include!("terminal_runtime_geometry.rs");
#[cfg(test)]
include!("terminal_runtime_test_support.rs");
