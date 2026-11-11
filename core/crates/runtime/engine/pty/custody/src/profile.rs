use super::CustodyDescriptorSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustodyProfile {
    ActiveRunning,
    ActiveTerminalReadOnly,
    ReleasedTerminal,
}

impl CustodyProfile {
    pub(crate) const fn for_descriptor_set(descriptor_set: CustodyDescriptorSet) -> Self {
        match descriptor_set {
            CustodyDescriptorSet::ReleasedSuccessor => Self::ReleasedTerminal,
            CustodyDescriptorSet::MacosProcessCapability { .. }
            | CustodyDescriptorSet::LinuxContainment => Self::ActiveRunning,
        }
    }

    pub const fn accepts_descriptor_set(self, descriptor_set: CustodyDescriptorSet) -> bool {
        matches!(
            (self, descriptor_set),
            (
                Self::ActiveRunning | Self::ActiveTerminalReadOnly,
                CustodyDescriptorSet::MacosProcessCapability { .. }
                    | CustodyDescriptorSet::LinuxContainment
            ) | (
                Self::ReleasedTerminal,
                CustodyDescriptorSet::ReleasedSuccessor
            )
        )
    }

    pub const fn is_active(self) -> bool {
        matches!(self, Self::ActiveRunning | Self::ActiveTerminalReadOnly)
    }
}
