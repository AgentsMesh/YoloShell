#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::time::timeout;
use uuid::Uuid;
use yolo_client::{Attachment, Client, ClientError};
use yolo_process_test_support as process;
use yolo_runtime::RuntimeRecord;
use yolo_runtime::cli::constants as cli;
use yolo_runtime::lease::constants::code as lease_error_code;
use yolo_runtime::lease::{LeaseEventPayload, LeaseReason};
use yolo_runtime::model::{
    CreateTerminal, ExitRecord, OutputChunk, TerminalRuntimeStatus, WriterCapability, WriterLease,
    WriterLeaseGrant,
};
use yolo_runtime::paths::{Paths, read_json};
use yolo_runtime::protocol::{
    EventEnvelope, ProtocolError, RequestEnvelope, ResponseEnvelope, delivery, error_code, event,
    field, from_value, method, read_frame, read_frame_blocking, to_value, write_frame,
    write_frame_blocking,
};
use yolo_runtime::worker::{
    AttachRequest, FencedTerminateRequest, InputRequest, LeaseRequest, RuntimeRef,
    WORKER_AUTH_METHOD, WORKER_STATUS_METHOD, WorkerAuth,
};

fn runtime_ref(terminal_id: Uuid, runtime_epoch: Uuid) -> RuntimeRef {
    RuntimeRef::new(terminal_id, runtime_epoch)
}

fn runtime_ref_from_record(record: &RuntimeRecord) -> RuntimeRef {
    runtime_ref(record.terminal_id, record.runtime_epoch)
}

fn runtime_ref_from_view(view: &yolo_runtime::TerminalView) -> RuntimeRef {
    runtime_ref(
        view.manifest.terminal_id,
        view.runtime
            .as_ref()
            .expect("running session runtime")
            .runtime_epoch,
    )
}

#[path = "fake_agent/constants.rs"]
mod fake_agent_constants;

#[path = "resilience/agent_recovery.rs"]
mod agent_recovery;
#[path = "resilience/child_cleanup.rs"]
mod child_cleanup;
#[path = "resilience/creation_capability.rs"]
mod creation_capability;
#[path = "resilience/creation_cleanup.rs"]
mod creation_cleanup;
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
#[path = "resilience/custody_release_recovery.rs"]
mod custody_release_recovery;
#[cfg(all(target_os = "macos", feature = "adversarial-test-hooks"))]
#[path = "resilience/guardian_fail_stop.rs"]
mod guardian_fail_stop;
#[path = "resilience/orphan_guard.rs"]
mod orphan_guard;
#[path = "resilience/process_groups.rs"]
mod process_groups;

#[path = "resilience/support/mod.rs"]
mod support;
use support::*;

#[path = "resilience/hub/mod.rs"]
mod hub;

#[path = "resilience/lifecycle.rs"]
mod lifecycle;
#[path = "resilience/multiplex.rs"]
mod multiplex;
#[path = "resilience/protocol.rs"]
mod protocol;
#[path = "resilience/protocol_ctl.rs"]
mod protocol_ctl;
#[path = "resilience/reboot.rs"]
mod reboot;
#[cfg(feature = "adversarial-test-hooks")]
#[path = "resilience/reconcile_panic.rs"]
mod reconcile_panic;
#[path = "resilience/recovery.rs"]
mod recovery;
#[path = "resilience/relay.rs"]
mod relay;
#[path = "resilience/relay_binary.rs"]
mod relay_binary;
#[path = "resilience/relay_completed.rs"]
mod relay_completed;
#[path = "resilience/relay_lease.rs"]
mod relay_lease;
#[path = "resilience/shell.rs"]
mod shell;
#[path = "resilience/shutdown.rs"]
mod shutdown;
#[path = "resilience/ssh_transport.rs"]
mod ssh_transport;
#[path = "resilience/terminal.rs"]
mod terminal;
#[path = "resilience/terminal_environment/mod.rs"]
mod terminal_environment;
#[cfg(feature = "adversarial-test-hooks")]
#[path = "resilience/termination_recovery.rs"]
mod termination_recovery;
#[path = "resilience/watchdog.rs"]
mod watchdog;
