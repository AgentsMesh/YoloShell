use super::PtySessionProof;
use super::linux_contract::BatchBudget;
use super::linux_fds::{self, DescriptorValue};
use crate::process::LiveProcessIdentity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::process) enum LinuxPtyHolderState {
    Excluded,
    Holds,
    Indeterminate,
}

impl PtySessionProof {
    pub(in crate::process) fn linux_holder_state(
        &self,
        identity: &LiveProcessIdentity,
    ) -> LinuxPtyHolderState {
        self.linux_holder_state_with_budget(identity, &mut BatchBudget::new())
    }

    pub(super) fn linux_holder_state_with_budget(
        &self,
        identity: &LiveProcessIdentity,
        budget: &mut BatchBudget,
    ) -> LinuxPtyHolderState {
        if !matches!(identity.try_matches(), Ok(true)) {
            return LinuxPtyHolderState::Indeterminate;
        }
        let query =
            linux_fds::holds_device(identity.pid(), self.tty_device, budget.descriptor_window());
        budget.charge_descriptors(query.examined);
        if !matches!(identity.try_matches(), Ok(true)) {
            return LinuxPtyHolderState::Indeterminate;
        }
        match query.value {
            DescriptorValue::Excluded => LinuxPtyHolderState::Excluded,
            DescriptorValue::Holds => LinuxPtyHolderState::Holds,
            DescriptorValue::Pending(_) => LinuxPtyHolderState::Indeterminate,
        }
    }
}
