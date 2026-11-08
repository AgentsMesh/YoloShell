pub(crate) struct ReapOutcome {
    pub(crate) status: Option<crate::process::DirectChildExitStatus>,
    pub(crate) fault: Option<anyhow::Error>,
    pub(crate) evidence: ReapEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ReapEvidence {
    Waited,
    LostUnknown,
}

impl ReapOutcome {
    pub(crate) fn waited(status: Option<crate::process::DirectChildExitStatus>) -> Self {
        Self {
            status,
            fault: None,
            evidence: ReapEvidence::Waited,
        }
    }

    pub(crate) fn lost_unknown() -> Self {
        Self {
            status: None,
            fault: None,
            evidence: ReapEvidence::LostUnknown,
        }
    }

    pub(crate) fn without_status(evidence: ReapEvidence) -> Self {
        match evidence {
            ReapEvidence::Waited => Self::waited(None),
            ReapEvidence::LostUnknown => Self::lost_unknown(),
        }
    }
}

pub(crate) enum FaultSettlement {
    Reaped(ReapOutcome),
    Deferred(anyhow::Error),
}
