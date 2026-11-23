#[cfg(target_os = "macos")]
const PROCESS_CAPABILITY_UNAVAILABLE_DETAIL: &str = "process capability proof is unavailable";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SessionOwnership {
    Tracking,
    OwnershipFault,
    Retired,
}

impl super::CapturedProcessTree {
    pub fn require_destructive_session(
        &self,
    ) -> std::result::Result<(), super::super::DestructiveSessionUnavailable> {
        self.destructive_authority().map(|_authority| ())
    }

    pub fn require_force_transaction_start(
        &self,
    ) -> std::result::Result<(), super::super::DestructiveSessionUnavailable> {
        if self.ownership != SessionOwnership::Tracking {
            return Err(super::super::DestructiveSessionUnavailable::new());
        }
        if !self.root_exit.is_gone() {
            self.require_destructive_session()?;
        }
        Ok(())
    }
}

#[cfg(target_os = "macos")]
impl super::CapturedProcessTree {
    pub fn install_process_capability(
        &mut self,
        authority: std::sync::Arc<crate::process::MacosProcessCapabilityManagerReader>,
    ) -> anyhow::Result<()> {
        authority.arm_liveness(self.live_root())?;
        self.pty_proof
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!(PROCESS_CAPABILITY_UNAVAILABLE_DETAIL))?
            .install_process_capability(authority)?;
        self.capability_scheduling_complete = false;
        self.pty_scan.restart();
        Ok(())
    }

    pub fn export_macos_secondary_custody(
        &self,
        runtime: &crate::pty_custody::CustodyRuntimeIdentity,
    ) -> anyhow::Result<crate::process::MacosSecondaryCustodyExport> {
        let proof = self
            .pty_proof
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!(PROCESS_CAPABILITY_UNAVAILABLE_DETAIL))?;
        crate::process::MacosSecondaryCustodyExport::capture(proof, runtime)
    }
}

#[cfg(target_os = "linux")]
impl super::CapturedProcessTree {
    pub fn export_linux_secondary_custody(
        &self,
        master: std::os::fd::RawFd,
    ) -> anyhow::Result<crate::process::LinuxSecondaryCustodyExport> {
        let containment = self.destructive.export_linux_containment_custody()?;
        crate::process::LinuxSecondaryCustodyExport::capture(master, containment)
    }
}

#[cfg(target_os = "linux")]
struct LinuxFrozenClosure {
    targets: Vec<crate::process::LiveProcessIdentity>,
    survivors: std::collections::HashSet<crate::process::LiveProcessIdentity>,
}

#[cfg(target_os = "linux")]
impl super::CapturedProcessTree {
    pub fn install_linux_containment(
        &mut self,
        containment: crate::process::linux_containment::LinuxContainmentLease,
    ) -> anyhow::Result<()> {
        anyhow::ensure!(
            containment.bound_root() == Some(self.root()),
            "Linux containment root does not match the captured PTY root"
        );
        self.destructive.install_linux_containment(containment)
    }

    pub(super) fn linux_containment_available(&self) -> bool {
        self.destructive.has_linux_containment()
    }

    pub(super) fn linux_frozen_closure(
        &mut self,
        request_freeze: bool,
    ) -> anyhow::Result<Option<Vec<crate::process::LiveProcessIdentity>>> {
        Ok(self
            .linux_frozen_observation(request_freeze)?
            .map(|observation| observation.targets))
    }

    fn linux_frozen_observation(
        &mut self,
        request_freeze: bool,
    ) -> anyhow::Result<Option<LinuxFrozenClosure>> {
        let Some(members) = self.destructive.poll_linux_frozen_members(request_freeze)? else {
            return Ok(None);
        };
        self.refresh()?;
        if self.descendant_observation() != crate::process::DescendantObservation::Settled {
            return Ok(None);
        }
        let targets = self.captured_targets();
        anyhow::ensure!(
            targets
                .iter()
                .all(|(identity, _)| members.contains(identity)),
            "captured PTY target escaped its exclusive cgroup"
        );
        let proof = self
            .pty_proof
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Linux containment has no PTY proof"))?;
        let target_set: std::collections::HashSet<_> =
            targets.iter().map(|(identity, _)| identity).collect();
        for member in &members {
            if !target_set.contains(member)
                && proof.linux_holder_state(member)
                    != crate::process::pty_session_proof::LinuxPtyHolderState::Excluded
            {
                return Ok(None);
            }
        }
        Ok(Some(LinuxFrozenClosure {
            targets: targets.into_iter().map(|(identity, _)| identity).collect(),
            survivors: members,
        }))
    }

    pub(super) fn prepare_linux_containment(
        &mut self,
    ) -> anyhow::Result<crate::process::linux_containment::LinuxContainmentPreparation> {
        let Some(observation) = self.linux_frozen_observation(false)? else {
            return Ok(
                crate::process::linux_containment::LinuxContainmentPreparation::MembershipChanged,
            );
        };
        if !observation.targets.is_empty() {
            return Ok(
                crate::process::linux_containment::LinuxContainmentPreparation::MembershipChanged,
            );
        }
        self.destructive
            .prepare_linux_containment(&observation.survivors)
    }

    pub(super) fn validate_linux_retained(
        &mut self,
        ready: &crate::process::linux_containment::LinuxContainmentCommitReady,
    ) -> anyhow::Result<bool> {
        self.destructive.validate_linux_retained(ready)
    }

    pub(super) fn linux_containment_ready_for_gone(&mut self) -> anyhow::Result<bool> {
        use crate::process::linux_containment::LinuxContainmentPreparation;
        match self.prepare_linux_containment()? {
            LinuxContainmentPreparation::MembershipChanged => Ok(false),
            LinuxContainmentPreparation::Removed => Ok(true),
            LinuxContainmentPreparation::Retained(ready) => self.validate_linux_retained(&ready),
        }
    }

    pub(super) fn settle_linux_after_master_release(&mut self) -> anyhow::Result<bool> {
        use crate::process::linux_containment::LinuxContainmentPreparation;
        if self.destructive.linux_containment_was_removed() {
            return Ok(true);
        }
        match self.prepare_linux_containment()? {
            LinuxContainmentPreparation::MembershipChanged => Ok(false),
            LinuxContainmentPreparation::Removed => Ok(true),
            LinuxContainmentPreparation::Retained(ready) => self
                .destructive
                .settle_linux_retained_after_master_release(&ready),
        }
    }

    pub fn requires_linux_containment_force(&self) -> bool {
        self.linux_containment_available()
    }
}
