use serde::{Deserialize, Serialize};

/// Implemented provider capabilities. A tier is a guaranteed floor derived from
/// these ports plus conformance; capabilities may exceed the declared floor.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProviderCapabilities {
    pub detect: bool,
    pub resume_exact: bool,
    pub lifecycle: bool,
    pub conversation: bool,
    pub submit: bool,
    pub interrupt: bool,
    pub recovery: bool,
}

impl ProviderCapabilities {
    pub const UNSUPPORTED: Self = Self {
        detect: false,
        resume_exact: false,
        lifecycle: false,
        conversation: false,
        submit: false,
        interrupt: false,
        recovery: false,
    };

    pub const FULL: Self = Self {
        detect: true,
        resume_exact: true,
        lifecycle: true,
        conversation: true,
        submit: true,
        interrupt: true,
        recovery: true,
    };

    /// Whether every capability required by `tier` is present.
    pub fn contains(self, required: Self) -> bool {
        (!required.detect || self.detect)
            && (!required.resume_exact || self.resume_exact)
            && (!required.lifecycle || self.lifecycle)
            && (!required.conversation || self.conversation)
            && (!required.submit || self.submit)
            && (!required.interrupt || self.interrupt)
            && (!required.recovery || self.recovery)
    }

    /// Highest complete tier implied by the present capabilities.
    pub fn floor(self) -> ProviderTier {
        [
            ProviderTier::L5,
            ProviderTier::L4,
            ProviderTier::L3,
            ProviderTier::L2,
            ProviderTier::L1,
            ProviderTier::L0,
        ]
        .into_iter()
        .find(|tier| self.contains(tier.required()))
        .expect("L0 has no required capabilities")
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderTier {
    L0,
    L1,
    L2,
    L3,
    L4,
    L5,
}

impl ProviderTier {
    /// Capabilities needed for this tier to be guaranteed.
    pub const fn required(self) -> ProviderCapabilities {
        match self {
            Self::L0 => ProviderCapabilities::UNSUPPORTED,
            Self::L1 => ProviderCapabilities {
                detect: true,
                resume_exact: true,
                ..ProviderCapabilities::UNSUPPORTED
            },
            Self::L2 => ProviderCapabilities {
                detect: true,
                resume_exact: true,
                lifecycle: true,
                ..ProviderCapabilities::UNSUPPORTED
            },
            Self::L3 => ProviderCapabilities {
                detect: true,
                resume_exact: true,
                lifecycle: true,
                conversation: true,
                ..ProviderCapabilities::UNSUPPORTED
            },
            Self::L4 => ProviderCapabilities {
                detect: true,
                resume_exact: true,
                lifecycle: true,
                conversation: true,
                submit: true,
                interrupt: true,
                ..ProviderCapabilities::UNSUPPORTED
            },
            Self::L5 => ProviderCapabilities::FULL,
        }
    }
}
