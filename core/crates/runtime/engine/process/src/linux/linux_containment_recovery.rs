use super::{
    LinuxContainmentBroker, LinuxContainmentError, LinuxContainmentLease,
    LinuxContainmentPreparation, LinuxContainmentSettlement,
};
use crate::process::pty_session_proof::{LinuxPtyCustodyScanner, LinuxPtyCustodyState};
use crate::process::{LiveProcessIdentity, ProcessIdentity, PtySessionProof};

const DELEGATION_MISSING_DETAIL: &str = "configured Linux cgroup delegation is missing";
const MEMBER_INDETERMINATE_DETAIL: &str = "frozen cgroup member custody is indeterminate";
const RECOVERY_PENDING_DETAIL: &str = "Linux containment recovery is still pending";
const UNTRUSTED_HOLDER_DETAIL: &str = "frozen cgroup has an unauthenticated PTY holder";

pub enum LinuxContainmentRecoveryPoll {
    Pending,
    Settled { leaf_removed: bool },
}

pub struct LinuxContainmentRecovery {
    state: RecoveryState,
    settlement: LinuxContainmentSettlement,
}

enum RecoveryState {
    Settled,
    Active {
        lease: LinuxContainmentLease,
        proof: Option<PtySessionProof>,
        observed_empty: bool,
    },
}

impl LinuxContainmentRecovery {
    pub fn open(
        record: super::LinuxContainmentRecord,
        terminal_id: uuid::Uuid,
        runtime_epoch: uuid::Uuid,
        root: ProcessIdentity,
    ) -> Result<Self, LinuxContainmentError> {
        let complete =
            record.validate_recovery(terminal_id, runtime_epoch, &super::super::boot_id(), root)?;
        let proof = if complete {
            let device = record
                .pty_device
                .ok_or(LinuxContainmentError::Identity(MEMBER_INDETERMINATE_DETAIL))?;
            let capability = record
                .pty_capability
                .clone()
                .ok_or(LinuxContainmentError::Identity(MEMBER_INDETERMINATE_DETAIL))?;
            Some(
                PtySessionProof::restore_linux(device, capability)
                    .map_err(|_| LinuxContainmentError::Identity(MEMBER_INDETERMINATE_DETAIL))?,
            )
        } else {
            None
        };
        let broker = LinuxContainmentBroker::discover()?.ok_or(
            LinuxContainmentError::Unavailable(DELEGATION_MISSING_DETAIL),
        )?;
        let lease = broker.reopen_recovery(record.clone())?;
        let settlement = LinuxContainmentSettlement::capture(
            &broker.root,
            lease.as_ref().map(|lease| &lease.leaf),
            record,
            root,
        )?;
        let state = match lease {
            Some(lease) => RecoveryState::Active {
                lease,
                proof,
                observed_empty: false,
            },
            None => RecoveryState::Settled,
        };
        Ok(Self { state, settlement })
    }

    pub fn into_settlement(self) -> Result<LinuxContainmentSettlement, LinuxContainmentError> {
        if !matches!(&self.state, RecoveryState::Settled) {
            return Err(LinuxContainmentError::Unavailable(RECOVERY_PENDING_DETAIL));
        }
        self.settlement.verify()?;
        Ok(self.settlement)
    }

    pub fn poll(&mut self) -> Result<LinuxContainmentRecoveryPoll, LinuxContainmentError> {
        let RecoveryState::Active {
            lease,
            proof,
            observed_empty,
        } = &mut self.state
        else {
            return Ok(LinuxContainmentRecoveryPoll::Settled { leaf_removed: true });
        };
        let Some(proof) = proof else {
            let Some(members) = lease.poll_frozen_members(true)? else {
                return Ok(LinuxContainmentRecoveryPoll::Pending);
            };
            if !members.is_empty() {
                *observed_empty = false;
                return Ok(LinuxContainmentRecoveryPoll::Pending);
            }
            if !*observed_empty {
                *observed_empty = true;
                return Ok(LinuxContainmentRecoveryPoll::Pending);
            }
            match lease.prepare_cleanup(&members)? {
                LinuxContainmentPreparation::MembershipChanged => {
                    *observed_empty = false;
                    return Ok(LinuxContainmentRecoveryPoll::Pending);
                }
                LinuxContainmentPreparation::Removed => {}
                LinuxContainmentPreparation::Retained(_ready) => {
                    return Ok(LinuxContainmentRecoveryPoll::Pending);
                }
            }
            self.state = RecoveryState::Settled;
            return Ok(LinuxContainmentRecoveryPoll::Settled { leaf_removed: true });
        };
        let Some(members) = lease.poll_frozen_members(true)? else {
            return Ok(LinuxContainmentRecoveryPoll::Pending);
        };
        let targets = classify_targets(proof, &members)?;
        if !targets.is_empty() {
            *observed_empty = false;
            for target in targets {
                super::super::signal_authority::signal_exact_process(
                    &target,
                    super::super::signal_authority::ExactProcessSignal::ForceTermination,
                )
                .map_err(|_| LinuxContainmentError::Identity(MEMBER_INDETERMINATE_DETAIL))?;
            }
            return Ok(LinuxContainmentRecoveryPoll::Pending);
        }
        if !*observed_empty {
            *observed_empty = true;
            return Ok(LinuxContainmentRecoveryPoll::Pending);
        }
        let leaf_removed = match lease.prepare_cleanup(&members)? {
            LinuxContainmentPreparation::MembershipChanged => {
                *observed_empty = false;
                return Ok(LinuxContainmentRecoveryPoll::Pending);
            }
            LinuxContainmentPreparation::Removed => true,
            LinuxContainmentPreparation::Retained(_ready) => {
                return Ok(LinuxContainmentRecoveryPoll::Pending);
            }
        };
        self.state = RecoveryState::Settled;
        Ok(LinuxContainmentRecoveryPoll::Settled { leaf_removed })
    }
}

fn classify_targets(
    proof: &PtySessionProof,
    members: &std::collections::HashSet<LiveProcessIdentity>,
) -> Result<Vec<LiveProcessIdentity>, LinuxContainmentError> {
    let mut scanner = LinuxPtyCustodyScanner::new(proof);
    let mut targets = Vec::new();
    for member in members {
        match scanner.classify(member) {
            LinuxPtyCustodyState::Target => targets.push(member.clone()),
            LinuxPtyCustodyState::Survivor => {}
            LinuxPtyCustodyState::UntrustedHolder => {
                return Err(LinuxContainmentError::Identity(UNTRUSTED_HOLDER_DETAIL));
            }
            LinuxPtyCustodyState::Indeterminate => {
                return Err(LinuxContainmentError::Identity(MEMBER_INDETERMINATE_DETAIL));
            }
        }
    }
    Ok(targets)
}
