use super::super::WorkerState;
#[cfg(test)]
use super::super::control_test_hooks::BlockingControlHook;
use super::super::pty_exit_observer::RootExitObserver;
use super::super::pty_output_progress::PtyOutputRecovery;
use super::super::pty_owner_command_channel::PtyOwnerCommandReceiver;
#[cfg(test)]
use super::super::pty_owner_handle::PtyOwnerCommand;
use super::super::pty_owner_handle::{OwnerSettlement, OwnerSettlementPublisher};
use super::super::pty_ownership::PtyOwnership;
use super::PtyOwner;
#[cfg(test)]
use std::sync::Arc;
use std::sync::Weak;
#[cfg(test)]
use std::sync::mpsc;
use std::time::Instant;

pub(crate) struct PtyOwnerCustody {
    #[cfg(test)]
    pub(crate) construction_panic: Option<&'static str>,
    pub(crate) ownership: PtyOwnership,
    #[cfg(test)]
    pub(crate) publication_barrier: Option<PublicationBarrier>,
    pub(crate) settlement_publisher: OwnerSettlementPublisher,
}

impl PtyOwnerCustody {
    pub(crate) fn new(
        ownership: PtyOwnership,
        settlement_publisher: OwnerSettlementPublisher,
    ) -> Self {
        Self {
            #[cfg(test)]
            construction_panic: None,
            ownership,
            #[cfg(test)]
            publication_barrier: None,
            settlement_publisher,
        }
    }

    pub(crate) fn output_recovery(&self) -> PtyOutputRecovery {
        #[cfg(test)]
        if let Some(detail) = self.construction_panic {
            panic!("{detail}");
        }
        self.ownership.output_recovery()
    }

    pub(crate) fn into_parts(self) -> (PtyOwnership, OwnerSettlementPublisher) {
        let Self {
            #[cfg(test)]
                construction_panic: _,
            ownership,
            #[cfg(test)]
            publication_barrier,
            settlement_publisher,
        } = self;
        #[cfg(test)]
        drop(publication_barrier);
        (ownership, settlement_publisher)
    }

    pub(crate) fn ownership(&self) -> &PtyOwnership {
        &self.ownership
    }

    pub(crate) fn ownership_mut(&mut self) -> &mut PtyOwnership {
        &mut self.ownership
    }

    #[cfg(test)]
    pub(crate) fn inject_construction_panic(&mut self, detail: &'static str) {
        self.construction_panic = Some(detail);
    }

    #[cfg(test)]
    pub(crate) fn block_publication_after_custody_drop(
        &mut self,
        barrier: Arc<BlockingControlHook>,
    ) {
        self.publication_barrier = Some(PublicationBarrier(barrier));
    }
}

impl PtyOwnership {
    pub(crate) fn into_owner_custody(
        self,
        settlement_publisher: OwnerSettlementPublisher,
    ) -> PtyOwnerCustody {
        PtyOwnerCustody::new(self, settlement_publisher)
    }

    #[cfg(test)]
    pub(crate) fn into_owner_for_test(
        self,
        state: Weak<WorkerState>,
        observer: Box<dyn RootExitObserver>,
        commands: mpsc::Receiver<PtyOwnerCommand>,
        settlement: OwnerSettlement,
    ) -> PtyOwner {
        let publisher = OwnerSettlementPublisher::for_test(&settlement);
        let commands = PtyOwnerCommandReceiver::from_uncoordinated_receiver(commands);
        PtyOwner::new_with_custody(
            state,
            self.into_owner_custody(publisher),
            observer,
            commands,
            settlement,
        )
    }
}

impl PtyOwner {
    pub(crate) fn new_with_custody(
        state: Weak<WorkerState>,
        custody: PtyOwnerCustody,
        observer: Box<dyn RootExitObserver>,
        commands: PtyOwnerCommandReceiver,
        settlement: OwnerSettlement,
    ) -> Self {
        let output_recovery = custody.output_recovery();
        let (ownership, settlement_publisher) = custody.into_parts();
        Self {
            state,
            ownership,
            output_recovery,
            observer,
            commands: Some(commands),
            pending: Vec::new(),
            termination: None,
            failure: None,
            next_refresh: Instant::now(),
            settlement,
            _settlement_publisher: settlement_publisher,
        }
    }
}

#[cfg(test)]
pub(crate) struct PublicationBarrier(Arc<BlockingControlHook>);

#[cfg(test)]
impl Drop for PublicationBarrier {
    fn drop(&mut self) {
        self.0.pause_if_armed();
    }
}
