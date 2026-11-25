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
