use super::{LinuxContainmentError, LinuxContainmentLease};
use crate::process::LiveProcessIdentity;
use std::collections::HashSet;

const SURVIVOR_IDENTITY_DETAIL: &str = "classified cgroup survivor identity is indeterminate";

#[derive(Debug)]
pub(crate) enum LinuxContainmentPreparation {
    MembershipChanged,
    Removed,
    Retained(LinuxContainmentCommitReady),
}

#[derive(Debug)]
pub(crate) struct LinuxContainmentCommitReady {
    survivors: HashSet<LiveProcessIdentity>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SettlementDecision {
    MembershipChanged,
    Removed,
    Retained,
}

impl LinuxContainmentLease {
    pub(crate) fn prepare_cleanup(
        &mut self,
        expected_survivors: &HashSet<LiveProcessIdentity>,
    ) -> Result<LinuxContainmentPreparation, LinuxContainmentError> {
        let Some(members) = self.poll_frozen_members(false)? else {
            return Ok(LinuxContainmentPreparation::MembershipChanged);
        };
        match settlement_for_members(expected_survivors, &members)? {
            SettlementDecision::MembershipChanged => {
                Ok(LinuxContainmentPreparation::MembershipChanged)
            }
            SettlementDecision::Removed => {
                super::descriptor::remove_exact_leaf(
                    &self.root,
                    &self.leaf,
                    &self.record.leaf_name,
                )?;
                self.freeze_requested_at = None;
                self.membership_unstable_since = None;
                Ok(LinuxContainmentPreparation::Removed)
            }
            SettlementDecision::Retained => Ok(LinuxContainmentPreparation::Retained(
                LinuxContainmentCommitReady { survivors: members },
            )),
        }
    }

    pub(crate) fn validate_retained(
        &mut self,
        ready: &LinuxContainmentCommitReady,
    ) -> Result<bool, LinuxContainmentError> {
        let Some(members) = self.poll_frozen_members(false)? else {
            return Ok(false);
        };
        Ok(settlement_for_members(&ready.survivors, &members)? == SettlementDecision::Retained)
    }
}

fn settlement_for_members(
    expected: &HashSet<LiveProcessIdentity>,
    observed: &HashSet<LiveProcessIdentity>,
) -> Result<SettlementDecision, LinuxContainmentError> {
    if observed != expected {
        return Ok(SettlementDecision::MembershipChanged);
    }
    for survivor in expected {
        let matches = survivor
            .try_matches()
            .map_err(|_| LinuxContainmentError::Identity(SURVIVOR_IDENTITY_DETAIL))?;
        if !matches {
            return Ok(SettlementDecision::MembershipChanged);
        }
    }
    Ok(if observed.is_empty() {
        SettlementDecision::Removed
    } else {
        SettlementDecision::Retained
    })
}

#[cfg(test)]
impl LinuxContainmentPreparation {
    pub(crate) fn retained_for_test() -> Self {
        Self::Retained(LinuxContainmentCommitReady {
            survivors: HashSet::new(),
        })
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/linux/linux_containment_settlement_tests.rs"]
mod tests;
