use super::constants::PROCESS_SCOPE_REFRESH_INTERVAL;
use super::pty_force_termination::{ForceOutput, ForceTerminationPoll};
use super::pty_missing_eof::MissingEofPoll;
use super::pty_output_progress::PtyOutputRecovery;
use super::pty_ownership::PtyOwnership;
use super::termination_timing::GRACEFUL_TERMINATION_TIMEOUT;
use super::termination_timing::{CapabilitySettlementDeadline, CapabilitySettlementDeferred};
use anyhow::Result;
use std::time::Instant;

pub(crate) struct OwnerTermination {
    pub(crate) force_requested: bool,
    pub(crate) graceful_deadline: Instant,
    pub(crate) next_signal: Instant,
    pub(crate) capability_deadline: Option<CapabilitySettlementDeadline>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TerminationPhase {
    Graceful,
    Force,
}

#[derive(Clone, Copy)]
pub(crate) enum TerminationAuthority {
    Available,
    #[cfg_attr(not(target_os = "linux"), allow(dead_code))]
    LinuxContainment,
    Unavailable,
}

impl OwnerTermination {
    pub(crate) fn new() -> Self {
        Self::new_at(Instant::now())
    }

    pub(crate) fn new_at(now: Instant) -> Self {
        Self {
            force_requested: false,
            graceful_deadline: now + GRACEFUL_TERMINATION_TIMEOUT,
            next_signal: now,
            capability_deadline: None,
        }
    }

    pub(crate) fn request(&mut self, force: bool) {
        self.request_at(force, Instant::now());
    }

    pub(crate) fn request_at(&mut self, force: bool, now: Instant) {
        self.force_requested |= force;
        if force {
            self.next_signal = now;
        }
    }

    pub(crate) fn poll(
        &mut self,
        ownership: &mut PtyOwnership,
        output: &mut PtyOutputRecovery,
    ) -> Result<ForceTerminationPoll> {
        self.poll_at(Instant::now(), ownership, output)
    }

    pub(crate) fn poll_at(
        &mut self,
        now: Instant,
        ownership: &mut PtyOwnership,
        output: &mut PtyOutputRecovery,
    ) -> Result<ForceTerminationPoll> {
        let authority = TerminationAuthority::capture(ownership);
        if self.phase_at(now, authority) == TerminationPhase::Force {
            let progress = ownership.poll_force(&mut ForceOutput::Reader(output))?;
            if matches!(progress, ForceTerminationPoll::Pending)
                && ownership.poll_missing_eof(output, now)? == MissingEofPoll::Lost
            {
                return Ok(ForceTerminationPoll::Pending);
            }
            let deadline = self
                .capability_deadline
                .get_or_insert_with(|| CapabilitySettlementDeadline::production(now));
            if matches!(progress, ForceTerminationPoll::Pending) && deadline.expired_at(now) {
                return Err(anyhow::Error::new(CapabilitySettlementDeferred));
            }
            return Ok(progress);
        }
        if now < self.next_signal {
            return Ok(ForceTerminationPoll::Pending);
        }
        self.next_signal = now + PROCESS_SCOPE_REFRESH_INTERVAL;
        ownership.tree_mut().request_graceful_termination()?;
        output.progress()?;
        Ok(ForceTerminationPoll::Pending)
    }

    pub(crate) fn phase_at(
        &mut self,
        now: Instant,
        authority: TerminationAuthority,
    ) -> TerminationPhase {
        self.force_requested |=
            matches!(authority, TerminationAuthority::Unavailable) || now >= self.graceful_deadline;
        if self.force_requested {
            TerminationPhase::Force
        } else {
            TerminationPhase::Graceful
        }
    }

    #[cfg(test)]
    pub(crate) fn expire_capability_deadline_for_test(&mut self) {
        self.capability_deadline = Some(CapabilitySettlementDeadline::immediate(Instant::now()));
    }
}

impl TerminationAuthority {
    pub(crate) fn capture(ownership: &mut PtyOwnership) -> Self {
        if ownership.require_destructive_session().is_err() {
            return Self::Unavailable;
        }
        #[cfg(target_os = "linux")]
        if ownership
            .tracking_tree_mut()
            .is_some_and(|tree| tree.requires_linux_containment_force())
        {
            // Containment authorizes eventual escalation; it does not request it.
            return Self::LinuxContainment;
        }
        Self::Available
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_termination_tests.rs"]
mod tests;
