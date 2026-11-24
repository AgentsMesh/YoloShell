use crate::process::{DirectChildIdentity, LiveProcessIdentity, ProcessIdentity};

pub(crate) enum StartupUntrackedIdentity {
    Exact {
        durable: ProcessIdentity,
        live: LiveProcessIdentity,
    },
    Incomplete {
        durable: Option<ProcessIdentity>,
        live: Option<LiveProcessIdentity>,
    },
}

impl StartupUntrackedIdentity {
    pub(crate) fn exact(durable: ProcessIdentity, live: LiveProcessIdentity) -> Self {
        Self::Exact { durable, live }
    }

    pub(crate) fn incomplete(
        durable: Option<ProcessIdentity>,
        live: Option<LiveProcessIdentity>,
    ) -> Self {
        Self::Incomplete { durable, live }
    }

    pub(crate) fn bind(&mut self, identity: DirectChildIdentity) {
        *self = Self::exact(identity.root(), identity.live_root());
    }

    pub(crate) fn durable(&self) -> Option<ProcessIdentity> {
        match self {
            Self::Exact { durable, .. } => Some(*durable),
            Self::Incomplete { durable, .. } => *durable,
        }
    }

    pub(crate) fn live(&self) -> Option<&LiveProcessIdentity> {
        match self {
            Self::Exact { live, .. } => Some(live),
            Self::Incomplete { live, .. } => live.as_ref(),
        }
    }

    #[cfg(test)]
    pub(crate) fn unavailable_for_test() -> Self {
        Self::incomplete(None, None)
    }

    #[cfg(test)]
    pub(crate) fn install_durable_for_test(&mut self, identity: ProcessIdentity) {
        *self = Self::Incomplete {
            durable: Some(identity),
            live: None,
        };
    }
}
