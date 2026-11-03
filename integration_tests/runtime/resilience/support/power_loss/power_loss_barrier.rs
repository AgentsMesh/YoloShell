use super::guardians::BoundCustody;
use super::*;
use anyhow::{Context, Result, ensure};
use std::collections::{HashMap, HashSet};
use yolo_process_test_support::{
    ProcessIdentity as RuntimeProcessIdentity, ProcessIdentityObservation,
};

#[path = "power_loss_barrier_authority.rs"]
mod authority;
#[path = "power_loss_barrier_capture.rs"]
mod capture;
#[path = "power_loss_barrier_drop.rs"]
mod drop_cleanup;
#[path = "power_loss_barrier_kill.rs"]
mod kill;
#[path = "power_loss_barrier_targets.rs"]
mod targets;
#[path = "power_loss_barrier_trees.rs"]
mod trees;
#[path = "power_loss_barrier_wait.rs"]
mod wait;
pub(super) use authority::{SessionAuthorityPhase, confirm_session_authority};

const TARGET_ALIAS_DETAIL: &str = "simulated power-loss process identity has multiple owners";
const TARGET_AUTHORITY_MISSING_DETAIL: &str =
    "simulated power-loss target has no runtime authority";
const TARGET_NOT_FROZEN_DETAIL: &str = "simulated power-loss target is not stably stopped";
const WORKER_KILL_ORDER: u8 = 0;
const GUARDIAN_KILL_ORDER: u8 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PowerLossTargetOwner {
    Worker(Uuid),
    Guardian(Uuid),
    RuntimeTree(Uuid),
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum PowerLossBarrierPhase {
    Freezing,
    Killing,
    Complete,
}

#[cfg(test)]
#[derive(Clone, Copy)]
pub(super) enum PowerLossCommitBoundary {
    RuntimeTree,
    DirectTarget,
}

#[cfg(test)]
impl PowerLossCommitBoundary {
    fn matches(self, owner: PowerLossTargetOwner) -> bool {
        matches!(
            (self, owner),
            (Self::RuntimeTree, PowerLossTargetOwner::RuntimeTree(_))
                | (
                    Self::DirectTarget,
                    PowerLossTargetOwner::Worker(_) | PowerLossTargetOwner::Guardian(_)
                )
        )
    }
}

pub(super) struct PowerLossFreezeBarrier {
    pub(super) workers: Vec<RuntimeProcessIdentity>,
    pub(super) guardians: Vec<BoundCustody>,
    trees: Vec<trees::PowerLossProcessTree>,
    pub(super) targets: HashMap<RuntimeProcessIdentity, PowerLossTargetOwner>,
    pub(super) frozen: HashSet<RuntimeProcessIdentity>,
    pub(super) thaw_on_drop: HashSet<RuntimeProcessIdentity>,
    pub(super) phase: PowerLossBarrierPhase,
}

impl PowerLossFreezeBarrier {
    pub(super) fn complete(&mut self) {
        self.phase = PowerLossBarrierPhase::Complete;
        self.frozen.clear();
        self.thaw_on_drop.clear();
    }

    pub(super) fn insert_new_target(
        &mut self,
        identity: RuntimeProcessIdentity,
        owner: PowerLossTargetOwner,
    ) -> Result<()> {
        ensure!(
            self.targets.insert(identity, owner).is_none(),
            "{TARGET_ALIAS_DETAIL}: {identity:?}"
        );
        Ok(())
    }

    pub(super) fn register_tree_target(
        &mut self,
        identity: RuntimeProcessIdentity,
        owner: PowerLossTargetOwner,
    ) -> Result<bool> {
        match self.targets.get(&identity) {
            Some(current) => {
                ensure!(*current == owner, "{TARGET_ALIAS_DETAIL}: {identity:?}");
                Ok(false)
            }
            None => {
                self.targets.insert(identity, owner);
                Ok(true)
            }
        }
    }

    fn record_for_owner<'a>(
        &self,
        records: &'a [BoundRuntimeRecord],
        owner: PowerLossTargetOwner,
    ) -> Result<&'a BoundRuntimeRecord> {
        let terminal_id = match owner {
            PowerLossTargetOwner::Worker(terminal_id)
            | PowerLossTargetOwner::Guardian(terminal_id)
            | PowerLossTargetOwner::RuntimeTree(terminal_id) => terminal_id,
        };
        records
            .iter()
            .find(|record| record.record().terminal_id == terminal_id)
            .context(TARGET_AUTHORITY_MISSING_DETAIL)
    }

    fn sorted_direct_targets(&self) -> Vec<RuntimeProcessIdentity> {
        self.sorted_direct_target_entries()
            .into_iter()
            .map(|(identity, _)| identity)
            .collect()
    }

    fn sorted_direct_target_entries(&self) -> Vec<(RuntimeProcessIdentity, PowerLossTargetOwner)> {
        let mut targets = self
            .targets
            .iter()
            .filter_map(|(identity, owner)| {
                (!matches!(owner, PowerLossTargetOwner::RuntimeTree(_)))
                    .then_some((*identity, *owner))
            })
            .collect::<Vec<_>>();
        targets.sort_by_key(|(identity, owner)| (owner_kill_key(*owner), identity_key(*identity)));
        targets
    }
}

fn owner_kill_key(owner: PowerLossTargetOwner) -> (u128, u8) {
    match owner {
        PowerLossTargetOwner::Worker(terminal_id) => (terminal_id.as_u128(), WORKER_KILL_ORDER),
        PowerLossTargetOwner::Guardian(terminal_id) => (terminal_id.as_u128(), GUARDIAN_KILL_ORDER),
        PowerLossTargetOwner::RuntimeTree(terminal_id) => (terminal_id.as_u128(), WORKER_KILL_ORDER),
    }
}

fn sort_targets(targets: &mut Vec<RuntimeProcessIdentity>) {
    targets.sort_by_key(|identity| identity_key(*identity));
}

fn identity_key(identity: RuntimeProcessIdentity) -> (u32, u64) {
    (identity.pid, identity.start_time)
}

#[cfg(test)]
#[path = "power_loss_barrier_tests.rs"]
mod tests;
