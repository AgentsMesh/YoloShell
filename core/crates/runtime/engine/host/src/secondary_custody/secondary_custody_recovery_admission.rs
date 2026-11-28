use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::host) enum SecondaryCustodyRecoveryAdmission {
    AwaitingSettlement,
    Retired,
    Unavailable,
}

pub(in crate::host) struct SecondaryCustodyRetirementEvidence {
    durable: super::recovery_evidence::DurableRetirementEvidence,
}

impl SecondaryCustodyRegistry {
    pub(in crate::host) fn recovery_admission(
        &self,
        host: &Host,
        runtime: &CustodyRuntimeIdentity,
    ) -> SecondaryCustodyRecoveryAdmission {
        let admission = self.admission(runtime);
        match admission {
            SecondaryCustodyAdmission::Active { lease, .. } if lease.runtime == *runtime => {
                SecondaryCustodyRecoveryAdmission::AwaitingSettlement
            }
            SecondaryCustodyAdmission::Retired {
                runtime: retired,
                state: SecondaryCustodyState::ReleaseAckPending,
            } if retired == *runtime => SecondaryCustodyRecoveryAdmission::AwaitingSettlement,
            SecondaryCustodyAdmission::Retired {
                state: SecondaryCustodyState::Released | SecondaryCustodyState::Superseded,
                ..
            }
            | SecondaryCustodyAdmission::Absent => {
                match super::recovery_evidence::DurableRetirementEvidence::capture(host, runtime) {
                    super::recovery_evidence::DurableRetirementObservation::Retired(_) => {
                        SecondaryCustodyRecoveryAdmission::Retired
                    }
                    super::recovery_evidence::DurableRetirementObservation::AwaitingSettlement => {
                        SecondaryCustodyRecoveryAdmission::AwaitingSettlement
                    }
                    super::recovery_evidence::DurableRetirementObservation::Unavailable
                        if super::recovery_evidence::settlement_is_pending(host, runtime) =>
                    {
                        SecondaryCustodyRecoveryAdmission::AwaitingSettlement
                    }
                    super::recovery_evidence::DurableRetirementObservation::Unavailable => {
                        SecondaryCustodyRecoveryAdmission::Unavailable
                    }
                }
            }
            SecondaryCustodyAdmission::Retired { .. } => {
                SecondaryCustodyRecoveryAdmission::Unavailable
            }
            SecondaryCustodyAdmission::Active { .. } => {
                SecondaryCustodyRecoveryAdmission::Unavailable
            }
        }
    }

    pub(in crate::host) fn retirement_evidence(
        &self,
        host: &Host,
        runtime: &CustodyRuntimeIdentity,
    ) -> Option<SecondaryCustodyRetirementEvidence> {
        match super::recovery_evidence::DurableRetirementEvidence::capture(host, runtime) {
            super::recovery_evidence::DurableRetirementObservation::Retired(durable) => {
                Some(SecondaryCustodyRetirementEvidence { durable })
            }
            super::recovery_evidence::DurableRetirementObservation::AwaitingSettlement
            | super::recovery_evidence::DurableRetirementObservation::Unavailable => None,
        }
    }
}

impl SecondaryCustodyRetirementEvidence {
    pub(in crate::host) fn verify(&self, host: &Host) -> anyhow::Result<()> {
        self.durable.verify(host)
    }
}
