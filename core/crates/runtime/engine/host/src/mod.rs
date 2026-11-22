use crate::agent::launch::{ResumeIntent, ResumePolicy, ShellCommand, shell_command};
use crate::agent::{AgentBinding, AgentBindingStore, AgentRegistry};
use crate::catalog::Catalog;
use crate::constants::{
    RUNTIME_RECORD_SCHEMA_VERSION, SESSION_MANIFEST_SCHEMA_VERSION, WORKER_LAUNCH_SCHEMA_VERSION,
};
use crate::coordination::CoordinationLock;
use crate::local_unix::LocalUnixStream as UnixStream;
use crate::model::{
    BootstrapDelivery, CleanupTerminationOutcome, CreateTerminal, CreateTerminalResult,
    CreationCleanupArchiveRequest, CreationCleanupDiscardRequest, CreationCleanupTerminateRequest,
    DEFAULT_COLS, DEFAULT_ROWS, ExitReason, ExitRecord, FinalizeCreationRequest, HostStatus,
    RecoveryState, RuntimeRecord, RuntimeView, TerminalDiscardRequest, TerminalLifecycleSelection,
    TerminalManifest, TerminalManifestDigest, TerminalRuntimeStatus, TerminalTerminateRequest,
    TerminalView, WorkerLaunchSpec, normalize_terminal_columns, normalize_terminal_rows, now_ms,
};
use crate::paths::{Paths, constants::NON_OWNER_PERMISSION_MASK, validate_home};
use crate::process;
use crate::protocol::{ErrorBody, RequestEnvelope, ResponseEnvelope, method};
use crate::worker::{
    AttachRequest, TerminalRef, WORKER_SHUTDOWN_METHOD, WORKER_STATUS_METHOD, WorkerRpcError,
    authenticated_stream, exchange_worker_request,
};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard, Weak};
use std::time::Duration;
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
use uuid::Uuid;
include!("module_wiring.rs");
mod rpc;
#[cfg(test)]
use crate::paths::identity::FileIdentity;
use agent_binding::{ResumeBinding, ResumeSelection};
pub use api::*;
#[cfg(test)]
use automatic_resume_test_probes::AutomaticResumeTestProbes;
use automatic_worker_crash_exit::*;
use catalog::catalog_rebuild;
#[cfg(test)]
use catalog::{
    CatalogTestProbes, catalog_durable_dir, catalog_durable_file, catalog_durable_identity,
};
use completed_attachment_constants::*;
pub use config::HostConfig;
use constants::*;
use creation::{
    CLEANUP_FINALIZED_UNAUTHORIZED, CLEANUP_READY_TRANSITION_INVARIANT,
    CLEANUP_SURVIVOR_ARCHIVE_UNAUTHORIZED, CleanupCreationState, CreationBinding,
    CreationCatalogCommitGate, CreationCatalogTimeoutDisposition, CreationEvidenceIdentity,
    CreationLaunchIdentity, CreationLedger, CreationPhase, CreationPublicationCommitInFlight,
    CreationPublicationDeadlineExpired, CreationReceiptStore, classify_creation_launch,
    creation_launch_uncertain, creation_publication_error, manifest_matches_binding,
    read_creation_evidence, require_creation_budget,
};
use ingress_constants::*;
pub use instance::HostInstanceBusy;
use instance::*;
#[cfg(test)]
use lifecycle::drain_host_clients;
use lifecycle::{LIFECYCLE_SELECTION_CHANGED, LifecycleMutationFence, validate_worker_attachment};
use reconcile::recovery_manifest_publication;
#[cfg(test)]
use reconcile::{
    CatalogRepairProbeOutcome, RECONCILIATION_FAILURE_COUNT, ReconcileRestoreProbe,
    ReconcileScanProbe, RecoveryPublicationProbe, contain_session_reconciliation,
    recoverable_runtime_loss,
};
#[cfg(any(test, feature = "failed-rearm-test-hooks"))]
#[cfg(any(not(test), feature = "yolo-test-support"))]
use resume::resume_terminal_rearm_test_hook;
use resume::{
    AutomaticResumeBlock, ProviderProcessScanCoordinator, RESUME_PROVIDER_BINDING_POLL_INTERVAL,
    RESUME_PROVIDER_BINDING_TIMEOUT, ResumeBindingDisposition, ResumeRetryDisposition,
    SessionResumeFailure,
};
use session::{
    SessionLocks, UnstartedLaunchPolicy, WORKER_START_DEADLINE_EXPIRED, lock_recover,
    spawn_host_worker_command_before,
};
use socket::*;
use worker::{
    WORKER_START_PUBLIC_DETAIL, WorkerRetirementControlFailure, WorkerRetirementControlPhase,
    WorkerRetirementError, WorkerRetirementObservation, WorkerStart, runtime_epochs_pair,
    runtime_view_matches, spawn_worker_reaper, validate_completed_snapshot, worker_connections,
    worker_start_failure, worker_unreachable,
};
use yolo_host_operation_deadline::HostOperationDeadline;
#[derive(Clone, Debug, Eq, PartialEq)]
struct PendingRecoveryWrite {
    to: RecoveryState,
    expected: TerminalManifestDigest,
}

pub struct Host {
    config: HostConfig,
    paths: Paths,
    socket: PathBuf,
    runtime_executable: PathBuf,
    catalog: Arc<Mutex<Catalog>>,
    creation_capabilities: Mutex<CreationLedger>,
    creation_receipts: CreationReceiptStore,
    attachment_clients: Arc<tokio::sync::Semaphore>,
    session_locks: SessionLocks,
    reconcile_gate: Arc<AsyncMutex<()>>,
    request_drain: request_drain::HostRequestDrain,
    parked_resume_retries: Mutex<HashMap<(Uuid, String), ResumeRetryDisposition>>,
    pending_recovery_writes: Mutex<HashMap<Uuid, PendingRecoveryWrite>>,
    secondary_custody: secondary_custody::SecondaryCustodyRegistry,
    secondary_custody_holder: crate::pty_custody::CustodyHolderIdentity,
    #[cfg(test)]
    catalog_test_probes: CatalogTestProbes,
    #[cfg(test)]
    automatic_resume_test_probes: AutomaticResumeTestProbes,
    watchdog_schedule: WatchdogSchedule,
    worker_status_timeout: Duration,
    worker_ready_timeout: Duration,
    worker_ready_poll_interval: Duration,
    resume_provider_binding_timeout: Duration,
    resume_provider_binding_poll_interval: Duration,
    provider_process_scanner: Arc<dyn crate::agent::ProcessScanner>,
    agent_registry: Arc<AgentRegistry>,
    provider_process_scans: ProviderProcessScanCoordinator,
    boot_id: String,
    controller_generation: u64,
    namespace: Arc<crate::namespace_lease::NamespaceLease>,
    socket_namespace: Arc<crate::paths::socket_namespace::SocketNamespace>,
    instance_ownership: HostInstanceOwnership,
    worker_connections: worker_connections::WorkerConnections,
    watch: agent_watch::HostWatch,
    workspace: yolo_host_workspace::WorkspaceService,
}
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/catalogue_watch_socket.rs"]
mod catalogue_watch_socket;
mod existing_presence;
#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/host_response_drain_socket.rs"]
#[cfg(test)]
mod response_drain_integration_tests;
#[cfg(test)]
#[path = "../test_support/tests.rs"]
mod tests;
pub use existing_presence::HostAbsence;

impl Host {
    fn agent_binding_store(&self, terminal_id: Uuid) -> AgentBindingStore {
        AgentBindingStore::with_provider_runtime(
            self.paths.agent_binding(terminal_id),
            Arc::clone(&self.config.provider_runtime),
        )
    }
}
