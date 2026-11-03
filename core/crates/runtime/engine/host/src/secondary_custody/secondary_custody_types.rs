use super::*;

#[cfg(target_os = "macos")]
type ActivePlatformSecondaryCustody = crate::process::MacosSecondaryCustody;
#[cfg(target_os = "linux")]
type ActivePlatformSecondaryCustody = crate::process::LinuxSecondaryCustody;

pub(super) enum PlatformSecondaryCustody {
    Active(ActivePlatformSecondaryCustody),
    ReleasedSuccessor,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(super) struct SecondaryCustodyKey {
    pub(super) terminal_id: Uuid,
    pub(super) runtime_epoch: Uuid,
    pub(super) worker_id: Uuid,
}

pub(in super::super) struct SecondaryCustodyLease {
    pub(super) runtime: CustodyRuntimeIdentity,
    pub(super) holder: CustodyHolderIdentity,
    pub(super) controller_generation: u64,
    pub(super) host_boot_id: String,
    pub(super) request_drain: HostRequestDrain,
    pub(super) offer: CustodyOffer,
    pub(super) guardian: Option<crate::pty_custody::CustodyGuardianPreparation>,
    #[cfg(target_os = "macos")]
    pub(super) durable_retirement: durable_retirement::SecondaryCustodyDurableRetirement,
    pub(super) platform: Mutex<Option<PlatformSecondaryCustody>>,
    pub(super) stream: AsyncMutex<Option<CustodyUnixStream>>,
    pub(super) monitor_commands: tokio::sync::mpsc::Sender<SecondaryCustodyCommand>,
    pub(super) monitor_receiver:
        Mutex<Option<tokio::sync::mpsc::Receiver<SecondaryCustodyCommand>>>,
    pub(super) registry: Mutex<Option<Weak<SecondaryCustodyEntries>>>,
    pub(super) state: Mutex<SecondaryCustodyState>,
    pub(super) state_events: tokio::sync::watch::Sender<SecondaryCustodyState>,
    pub(super) wire_ready: AtomicBool,
    pub(super) available: AtomicBool,
    pub(super) settlement_active: AtomicBool,
    #[cfg(test)]
    pub(super) worker_identity_probe: Mutex<Option<crate::process::ProcessIdentity>>,
}

pub(super) struct SecondaryCustodyEntries {
    pub(super) values: Mutex<SecondaryCustodyValues>,
    pub(super) gates: SessionLocks,
}

impl Default for SecondaryCustodyEntries {
    fn default() -> Self {
        Self {
            values: Mutex::new(SecondaryCustodyValues::default()),
            gates: SessionLocks::default(),
        }
    }
}

#[derive(Default)]
pub(super) struct SecondaryCustodyValues {
    pub(super) active: HashMap<SecondaryCustodyKey, Arc<SecondaryCustodyLease>>,
    pub(super) active_succession:
        HashMap<SecondaryCustodyKey, retirement::SecondaryCustodySuccessionOrdinal>,
    pub(super) retired: HashMap<SecondaryCustodyKey, retirement::SecondaryCustodyRetirement>,
    pub(super) succession: retirement::SecondaryCustodySuccession,
}

#[derive(Default)]
pub(in super::super) struct SecondaryCustodyRegistry {
    pub(super) entries: Arc<SecondaryCustodyEntries>,
    #[cfg(test)]
    pub(super) rpc_test_authority: Mutex<Vec<CustodyRuntimeIdentity>>,
    #[cfg(test)]
    pub(super) peer_test_authority:
        Mutex<Vec<(CustodyRuntimeIdentity, crate::process::ProcessIdentity)>>,
}

pub(super) enum SecondaryCustodyCommand {
    Supersede {
        successor: CustodyOffer,
        successor_generation: u64,
        completion: tokio::sync::oneshot::Sender<Result<(), WorkerRpcError>>,
    },
    #[cfg(test)]
    Panic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum SecondaryCustodyState {
    Preparing,
    RollingBack,
    Activating,
    Ready,
    ReleaseAckPending,
    Released,
    ReleaseUncertain,
    Superseded,
    Disconnected,
    Failed,
}

pub(super) enum SecondaryCustodyAdmission {
    Active {
        lease: Arc<SecondaryCustodyLease>,
        state: SecondaryCustodyState,
        ready: bool,
    },
    Retired {
        runtime: CustodyRuntimeIdentity,
        state: SecondaryCustodyState,
    },
    Absent,
}

impl SecondaryCustodyKey {
    pub(super) fn from_runtime(runtime: &CustodyRuntimeIdentity) -> Self {
        Self {
            terminal_id: runtime.terminal_id,
            runtime_epoch: runtime.runtime_epoch,
            worker_id: runtime.worker_id,
        }
    }
}
