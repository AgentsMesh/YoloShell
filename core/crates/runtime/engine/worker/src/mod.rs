pub(in crate::worker) use yolo_agent_launch_fence as agent_launch_fence;
mod agent_monitor;
mod attachment;
mod bootstrap;
mod pty;
mod reader;
mod secondary_custody;
mod startup;
mod runtime;
mod state;
mod claude_hook;
mod codex_hook;
mod constants;
mod controller_generation_persistence;
mod finalization_persistence;
mod finalization_reader;
mod input_delivery;
mod instance_lock;
mod local_transport;
mod namespace;
#[cfg(unix)]
mod nonblocking_pty;
mod output_block;
mod ownership_fault;
mod presentation;
mod protocol_admission;
pub use presentation::load_presentation;
#[cfg(test)]
#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))]
#[path = "../test_support/integration/pty/pty_runtime_alias_test_runtime.rs"]
mod pty_runtime_alias_test_runtime;
mod rpc;
pub use rpc::WorkerConnection;
pub(crate) use bootstrap::*;
pub(crate) use attachment::*;
pub(crate) use pty::*;
pub(crate) use reader::*;
pub(crate) use startup::*;
pub(crate) use state::*;
pub(crate) use runtime::*;
pub(crate) use secondary_custody::*;
pub(crate) use agent_monitor::{
    AgentMonitorConfig, agent_monitor_publication_fence, prepare_agent_monitor,
    refresh_agent_title, spawn_agent_monitor_task, supervise_agent_monitor,
};
#[cfg(test)]
pub(crate) use agent_monitor::{build_agent_monitor, persist_agent_detection};
mod request_types;
mod response_drain;
mod supervision;
mod terminal_reporting;
mod termination_timing;
include!("module_wiring.rs");
use bootstrap::bootstrap_gate::{BootstrapChildSpec, BootstrapExecConfirmation, accept_bootstrap_gate};
use self::claude_hook::ClaudeHookServer;
use self::constants::*;
use self::input_delivery::{CachedInputDelivery, CachedInputLookup, InputDeliveryCache};
use self::instance_lock::WorkerInstanceLock;
use self::namespace::WorkerNamespace;
#[cfg(unix)]
use self::nonblocking_pty::{PtyReadiness, configure_nonblocking_pty, wait_for_pty_readiness};
use self::output_block::{OutputBlock, OutputBlockCause};
use self::pty_input_writer::PtyInputWriter;
use self::pty_output_progress::ExclusivePtyOutput;
#[cfg(test)]
use self::pty_reader_activation::spawn_reader;
use self::pty_reader_activation::stage_reader;
use self::reader_lifecycle::{PtyReadOutcome, ReaderLifecycle, ReaderSupervisorCustody};
use self::reader_restart::{ReaderRecoveryBudget, ReaderRestartKind, ReaderRestartPhase};
use runtime::runtime_result::settle as settle_runtime_result;
use self::secondary_custody_lease::WorkerCustodyLease;
use self::secondary_custody_source::WorkerSecondaryCustodySource;
use self::secondary_custody_startup::establish as establish_secondary_custody;
use startup::startup_cleanup_scope::StartupCleanupScope;
use startup::startup_guard::StartupChildGuard;
use startup::startup_resources::StartupPtyResources;
use self::termination_timing::*;
use crate::agent::{
    AgentBinding, AgentBindingStore, AgentDetector, AgentRegistry, DetectionContext,
    DetectionResult, ProcessSource, binding_display_title_with_registry, detect_scan,
};
use crate::constants::{RUNTIME_RECORD_SCHEMA_VERSION, WORKER_LAUNCH_SCHEMA_VERSION};
use crate::journal::{
    AppendAttempt, Journal, JournalError, RetainedAppendBatchOutcome, read_snapshot,
};
use crate::lease::constants::code as lease_error_code;
use crate::lease::{
    LeaseAuthority, LeaseEventPayload, LeaseReason, LeaseTransition, LeaseTransitionKind,
};
use crate::model::{
    BootstrapDelivery, ExitReason, ExitRecord, OutputChunk, OutputGapPayload, RuntimeRecord,
    RuntimeView, TerminalRuntimeStatus, TerminalSnapshot, TerminationTombstone, WorkerHello,
    WorkerLaunchSpec, WriterCapability, WriterLease, WriterLeaseGrant, normalize_terminal_columns,
    normalize_terminal_rows, now_ms,
};
#[cfg(test)]
use crate::paths::atomic_write_json;
use crate::paths::{Paths, read_json};
use crate::protocol::{
    ErrorBody, EventEnvelope, ProtocolError, RequestEnvelope, ResponseEnvelope, event, from_value,
    read_frame, to_value, validate_response, write_frame,
};
use crate::{local_unix::LocalUnixStream as UnixStream, process};
use anyhow::{Context, Result, bail};
pub use bootstrap::run_bootstrap_child;
pub use runtime::run;
pub use attachment::attachment_next_sequence;
pub use constants::{WORKER_AUTH_METHOD, WORKER_SHUTDOWN_METHOD, WORKER_STATUS_METHOD};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::collections::HashSet;
use std::future::Future;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use yolo_agent_providers::ProviderRuntimeRegistry;
use std::time::{Duration, Instant};
use tokio::net::UnixListener;
use tokio::sync::{Notify, Semaphore, broadcast};
use tokio::task::JoinSet;
use uuid::Uuid;

include!("runtime_module_wiring.rs");
