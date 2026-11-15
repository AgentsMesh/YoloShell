use super::termination_timing::CapabilitySettlementDeferred;
use super::{StartupChildGuard, WorkerState, lock_recover};
use tokio::sync::oneshot;

#[path = "pty_owner_handle_commands.rs"]
mod commands;
#[path = "pty_owner_fault_observer.rs"]
mod fault_observer;
#[path = "pty_owner_settlement.rs"]
mod settlement;
#[path = "pty_owner_handle_staging.rs"]
mod staging;

pub(crate) use fault_observer::bind_guarded_fault_reap_observer;
#[cfg(test)]
pub(crate) use settlement::new_owner_settlement_for_test;
pub(crate) use settlement::{OwnerSettlement, OwnerSettlementPublisher, new_owner_settlement};
#[cfg(test)]
pub(crate) use staging::spawn_guarded_owner;
pub(crate) use staging::{StagedPtyOwner, stage_guarded_owner};

pub(crate) type CommandCompletion = std::result::Result<(), PtyOwnerCommandFailure>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum PtyOwnerCommandFailure {
    DestructiveUnavailable(crate::process::DestructiveSessionUnavailable),
    CleanupDeferred(CapabilitySettlementDeferred),
    Owner(String),
}

#[cfg(test)]
impl PtyOwnerCommandFailure {
    pub(crate) fn contains(&self, detail: &str) -> bool {
        match self {
            Self::DestructiveUnavailable(error) => error.to_string().contains(detail),
            Self::CleanupDeferred(error) => error.to_string().contains(detail),
            Self::Owner(message) => message.contains(detail),
        }
    }
}

#[derive(Clone)]
pub(crate) struct PtyOwnerHandle {
    pub(crate) commands: super::pty_owner_command_channel::PtyOwnerCommandSender,
    pub(crate) settlement: OwnerSettlement,
    pub(crate) destructive: DestructiveCapability,
}

pub(crate) enum PtyOwnerCommand {
    Terminate {
        force: bool,
        response: oneshot::Sender<CommandCompletion>,
    },
}

pub(crate) struct PtyTerminationSubmission {
    pub(crate) completion: Option<oneshot::Receiver<CommandCompletion>>,
    pub(crate) settlement: OwnerSettlement,
}

#[derive(Clone)]
pub(crate) enum DestructiveCapability {
    Available,
    Unavailable(crate::process::DestructiveSessionUnavailable),
}

impl PtyOwnerHandle {
    #[cfg(test)]
    pub(crate) fn for_test(
        commands: std::sync::mpsc::Sender<PtyOwnerCommand>,
        settlement: OwnerSettlement,
    ) -> Self {
        Self {
            commands:
                super::pty_owner_command_channel::PtyOwnerCommandSender::from_uncoordinated_sender(
                    commands,
                ),
            settlement,
            destructive: DestructiveCapability::Available,
        }
    }

    #[cfg(test)]
    pub(crate) fn unavailable_for_test(
        commands: std::sync::mpsc::Sender<PtyOwnerCommand>,
        settlement: OwnerSettlement,
    ) -> Self {
        Self {
            commands:
                super::pty_owner_command_channel::PtyOwnerCommandSender::from_uncoordinated_sender(
                    commands,
                ),
            settlement,
            destructive: DestructiveCapability::Unavailable(
                crate::process::DestructiveSessionUnavailable::new(),
            ),
        }
    }
}

impl WorkerState {
    pub(crate) fn pty_owner_handle(&self) -> Option<PtyOwnerHandle> {
        lock_recover(&self.pty_owner).clone()
    }

    pub(crate) fn install_pty_owner(&self, owner: PtyOwnerHandle) {
        let previous = lock_recover(&self.pty_owner).replace(owner);
        debug_assert!(previous.is_none());
    }

    #[cfg(test)]
    pub(crate) fn install_pty_owner_for_test(&self, owner: PtyOwnerHandle) {
        self.install_pty_owner(owner);
    }
}
