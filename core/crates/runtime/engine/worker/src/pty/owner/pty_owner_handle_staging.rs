use super::super::constants::PTY_OWNER_THREAD_PREFIX;
#[cfg(test)]
use super::super::control_test_hooks::BlockingControlHook;
use super::super::pty_exit_observer::WaitidRootExitObserver;
use super::super::pty_owner::{PtyOwner, PtyOwnerCustody};
use super::super::pty_owner_command_channel::{PtyOwnerCommandReceiver, pty_owner_command_channel};
use super::super::{StartupChildGuard, WorkerState, supervision};
use super::{
    DestructiveCapability, OwnerSettlement, OwnerSettlementPublisher, PtyOwnerHandle,
    bind_guarded_fault_reap_observer, new_owner_settlement,
};
use anyhow::{Context, Result, anyhow, bail};
use std::io;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, mpsc};

pub(crate) const OWNER_CHANNEL_CAPACITY: usize = 1;
pub(crate) const OWNER_THREAD_SPAWN_CONTEXT: &str = "spawn PTY lifetime owner thread";
pub(crate) const OWNER_TRANSFER_CLOSED_CONTEXT: &str =
    "PTY owner stopped before accepting ownership";
pub(crate) const OWNER_ACTIVATION_CLOSED_CONTEXT: &str =
    "PTY owner stopped before confirming construction";
pub(crate) const OWNER_CONSTRUCTION_PANIC_CONTEXT: &str = "PTY owner construction panicked";
pub(crate) const SETTLEMENT_PUBLISHER_INVARIANT: &str =
    "staged PTY owner retains settlement publisher until activation";
pub(crate) const TRACKED_ROOT_MISSING_CONTEXT: &str = "tracked PTY root is unavailable";

pub(crate) type OwnerActivation = std::result::Result<(), String>;
pub(crate) type OwnerThread = Box<dyn FnOnce() + Send + 'static>;

pub(crate) struct StagedPtyOwner {
    pub(crate) ownership: mpsc::SyncSender<PtyOwnerCustody>,
    pub(crate) activation: mpsc::Receiver<OwnerActivation>,
    pub(crate) handle: PtyOwnerHandle,
    pub(crate) state: Arc<WorkerState>,
    pub(crate) settlement_publisher: Option<OwnerSettlementPublisher>,
    #[cfg(test)]
    pub(crate) construction_panic: Option<&'static str>,
    #[cfg(test)]
    pub(crate) publication_barrier: Option<Arc<BlockingControlHook>>,
}

#[cfg(test)]
pub(crate) fn spawn_guarded_owner(
    guard: &mut StartupChildGuard,
    state: Arc<WorkerState>,
) -> Result<()> {
    stage_guarded_owner(guard, state)?.activate(guard)
}

pub(crate) fn stage_guarded_owner(
    guard: &mut StartupChildGuard,
    state: Arc<WorkerState>,
) -> Result<StagedPtyOwner> {
    bind_guarded_fault_reap_observer(guard, &state)?;
    guard.prepare_ownership_transfer()?;
    let destructive = match guard.destructive_session_capability() {
        Ok(()) => DestructiveCapability::Available,
        Err(error) => DestructiveCapability::Unavailable(error),
    };
    let root = guard.identity().context(TRACKED_ROOT_MISSING_CONTEXT)?;
    let (command_sender, command_receiver) = pty_owner_command_channel();
    let (settlement, publisher) = new_owner_settlement();
    let handle = PtyOwnerHandle {
        commands: command_sender,
        settlement: Arc::clone(&settlement),
        destructive,
    };
    let (ownership_sender, ownership_receiver) = mpsc::sync_channel(OWNER_CHANNEL_CAPACITY);
    let (activation_sender, activation_receiver) = mpsc::sync_channel(OWNER_CHANNEL_CAPACITY);
    let owner_state = Arc::downgrade(&state);
    let thread_name = format!("{}-{}", PTY_OWNER_THREAD_PREFIX, state.terminal_id());
    let publisher = spawn_owner_thread(thread_name, publisher, move || {
        let Ok(custody) = ownership_receiver.recv() else {
            return;
        };
        construct_and_run_owner(
            custody,
            owner_state,
            root,
            command_receiver,
            settlement,
            activation_sender,
        );
    })
    .context(OWNER_THREAD_SPAWN_CONTEXT)?;
    Ok(StagedPtyOwner {
        ownership: ownership_sender,
        activation: activation_receiver,
        handle,
        state,
        settlement_publisher: Some(publisher),
        #[cfg(test)]
        construction_panic: None,
        #[cfg(test)]
        publication_barrier: None,
    })
}

impl StagedPtyOwner {
    pub(crate) fn activate(mut self, guard: &mut StartupChildGuard) -> Result<()> {
        let ownership = guard.take_prepared_ownership();
        let publisher = self
            .settlement_publisher
            .take()
            .expect(SETTLEMENT_PUBLISHER_INVARIANT);
        let custody = ownership.into_owner_custody(publisher);
        #[cfg(test)]
        let mut custody = custody;
        #[cfg(test)]
        if let Some(detail) = self.construction_panic {
            custody.inject_construction_panic(detail);
        }
        #[cfg(test)]
        if let Some(barrier) = self.publication_barrier.take() {
            custody.block_publication_after_custody_drop(barrier);
        }
        if let Err(error) = self.ownership.send(custody) {
            drop(error.0);
            bail!(OWNER_TRANSFER_CLOSED_CONTEXT);
        }
        match self
            .activation
            .recv()
            .context(OWNER_ACTIVATION_CLOSED_CONTEXT)?
        {
            Ok(()) => {
                self.state.install_pty_owner(self.handle);
                Ok(())
            }
            Err(detail) => Err(anyhow!("{OWNER_CONSTRUCTION_PANIC_CONTEXT}: {detail}")),
        }
    }
}

pub(crate) fn construct_and_run_owner(
    custody: PtyOwnerCustody,
    state: std::sync::Weak<WorkerState>,
    root: crate::process::ProcessIdentity,
    commands: PtyOwnerCommandReceiver,
    settlement: OwnerSettlement,
    activation: mpsc::SyncSender<OwnerActivation>,
) {
    let construction = catch_unwind(AssertUnwindSafe(|| {
        PtyOwner::new_with_custody(
            state,
            custody,
            Box::new(WaitidRootExitObserver::new(root)),
            commands,
            settlement,
        )
    }));
    match construction {
        Ok(owner) => {
            let _ = activation.send(Ok(()));
            owner.run();
        }
        Err(payload) => {
            let _ = activation.send(Err(supervision::panic_detail(payload)));
        }
    }
}

pub(crate) fn spawn_owner_thread(
    name: String,
    publisher: OwnerSettlementPublisher,
    run: impl FnOnce() + Send + 'static,
) -> io::Result<OwnerSettlementPublisher> {
    spawn_owner_thread_with(name, publisher, run, |builder, owner| {
        builder.spawn(owner).map(drop)
    })
}

pub(crate) fn spawn_owner_thread_with<Run, Spawn>(
    name: String,
    publisher: OwnerSettlementPublisher,
    run: Run,
    spawn: Spawn,
) -> io::Result<OwnerSettlementPublisher>
where
    Run: FnOnce() + Send + 'static,
    Spawn: FnOnce(std::thread::Builder, OwnerThread) -> io::Result<()>,
{
    spawn(std::thread::Builder::new().name(name), Box::new(run))?;
    Ok(publisher)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../../../test_support/integration/pty/pty_owner_handle_staging_test_support.rs"]
mod test_support;
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/owner/pty_owner_handle_staging_unit_tests.rs"]
mod tests;
