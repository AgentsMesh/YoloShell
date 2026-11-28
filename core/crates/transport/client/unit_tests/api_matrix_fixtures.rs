use super::api_matrix_fixture_values::*;
use super::*;
use crate::model::{
    BootstrapDelivery, RecoveryState, RuntimeView, TerminalManifest, TerminalRuntimeStatus, TerminalSnapshot,
};
use crate::protocol::{RequestEnvelope, ResponseEnvelope, method};
#[derive(Clone, Copy)]
pub(super) enum MatrixScenario {
    Standard,
    RuntimeUnavailable,
    MismatchedSnapshot,
    IncompleteCatalogInventory,
    LegacyResumeHost,
}

pub(super) fn response(request: &RequestEnvelope, scenario: MatrixScenario) -> ResponseEnvelope {
    match request.method.as_str() {
        method::HOST_STATUS => success(request, &host_status()),
        method::LIST_SESSIONS => success(
            request,
            &ListSessionsResult {
                sessions: vec![session_view(scenario)],
                complete: !matches!(scenario, MatrixScenario::IncompleteCatalogInventory),
            },
        ),
        method::RESUME_SESSION_EXACT if matches!(scenario, MatrixScenario::LegacyResumeHost) => {
            ResponseEnvelope::failure(
                request,
                ErrorBody::new(
                    crate::protocol::error_code::UNSUPPORTED_METHOD,
                    LEGACY_EXACT_RESUME_UNSUPPORTED,
                    false,
                ),
            )
        }
        method::GET_SESSION
        | method::ARCHIVE_SESSION
        | method::DISCARD_SESSION
        | method::CLEANUP_ARCHIVE_SESSION
        | method::CLEANUP_DISCARD_SESSION
        | method::RESUME_SESSION_EXACT => success(request, &session_view(scenario)),
        method::CREATE_SESSION => success(request, &creation_result()),
        method::RECONCILE => success(request, &ReconcileResult::default()),
        method::ACQUIRE_WRITER | method::TAKEOVER_WRITER | method::RENEW_WRITER => {
            success(request, &lease_grant(request))
        }
        method::GET_WRITER_LEASE => success(request, &Some(lookup_lease())),
        method::TERMINAL_INPUT => input_response(request),
        method::GET_SNAPSHOT => success(request, &terminal_snapshot(scenario)),
        method::CLEANUP_TERMINATE_SESSION => {
            success(request, &CleanupTerminationOutcome::Terminated)
        }
        method::TERMINATE_SESSION
        | method::FINALIZE_CREATION
        | method::RELEASE_WRITER
        | method::RESIZE_TERMINAL
        | method::SIGNAL_SESSION => success(request, &()),
        other => panic!("unexpected client API matrix method {other}"),
    }
}

fn input_response(request: &RequestEnvelope) -> ResponseEnvelope {
    let input: InputRequest = request.decode_payload().unwrap();
    success(
        request,
        &InputResult {
            input_id: input.input_id,
            duplicate: false,
            delivery: TEST_DELIVERY.into(),
        },
    )
}

fn success<T: Serialize>(request: &RequestEnvelope, payload: &T) -> ResponseEnvelope {
    ResponseEnvelope::success(request, payload).unwrap()
}

pub(super) fn test_session_id() -> Uuid {
    Uuid::from_u128(TEST_SESSION_ID_VALUE)
}

pub(super) fn test_runtime_epoch() -> Uuid {
    Uuid::from_u128(TEST_RUNTIME_EPOCH_VALUE)
}

pub(super) fn test_manifest_digest() -> crate::model::TerminalManifestDigest {
    crate::model::TerminalManifestDigest::capture(&session_view(MatrixScenario::Standard).manifest)
        .unwrap()
}

pub(super) fn test_target() -> RuntimeRef {
    RuntimeRef::new(test_session_id(), test_runtime_epoch())
}

pub(super) fn test_creation_capability() -> Uuid {
    Uuid::from_u128(TEST_CREATION_CAPABILITY_VALUE)
}

pub(super) fn test_input_id() -> Uuid {
    Uuid::from_u128(TEST_INPUT_ID_VALUE)
}

fn lease_grant(request: &RequestEnvelope) -> WriterLeaseGrant {
    let request: LeaseRequest = request.decode_payload().unwrap();
    WriterLeaseGrant {
        lease: WriterLease {
            holder_id: TEST_GRANTED_HOLDER.into(),
            controller_instance_id: Some(TEST_GRANTED_CONTROLLER.into()),
            generation: TEST_LEASE_GENERATION,
            expires_at_ms: TEST_LEASE_EXPIRY_MS,
            ttl_ms: TEST_LEASE_TTL_MS,
            renew_after_ms: TEST_LEASE_RENEW_AFTER_MS,
        },
        writer_capability: request.writer_capability,
    }
}

fn lookup_lease() -> WriterLease {
    WriterLease {
        holder_id: TEST_LOOKUP_HOLDER.into(),
        controller_instance_id: None,
        generation: TEST_LEASE_GENERATION,
        expires_at_ms: TEST_LEASE_EXPIRY_MS,
        ttl_ms: TEST_LEASE_TTL_MS,
        renew_after_ms: TEST_LEASE_RENEW_AFTER_MS,
    }
}

fn creation_result() -> CreateTerminalResult {
    CreateTerminalResult {
        terminal_id: test_session_id(),
        creation_capability: test_creation_capability(),
    }
}

fn session_view(scenario: MatrixScenario) -> TerminalView {
    TerminalView {
        manifest: TerminalManifest {
            schema_version: yolo_runtime::constants::SESSION_MANIFEST_SCHEMA_VERSION,
            terminal_id: test_session_id(),
            name: TEST_SESSION_NAME.into(),
            command: TEST_SESSION_COMMAND.into(),
            args: Vec::new(),
            cwd: PathBuf::from(TEST_MANIFEST_CWD),
            cols: TEST_MANIFEST_COLUMNS,
            rows: TEST_MANIFEST_ROWS,
            created_at_ms: TEST_CREATED_AT_MS,
            updated_at_ms: TEST_UPDATED_AT_MS,
            archived: false,
            recovery_state: RecoveryState::Created,
        },
        runtime: (!matches!(scenario, MatrixScenario::RuntimeUnavailable)).then(|| RuntimeView {
            presentation: Default::default(),
            runtime_epoch: test_runtime_epoch(),
            worker_id: Uuid::from_u128(TEST_WORKER_ID_VALUE),
            worker_pid: TEST_WORKER_PROCESS_ID,
            worker_start_time: TEST_WORKER_START_TIME,
            child_pid: TEST_CHILD_PROCESS_ID,
            child_start_time: TEST_CHILD_START_TIME,
            status: TerminalRuntimeStatus::Running,
            last_output_sequence: TEST_LAST_OUTPUT_SEQUENCE,
            exit: None,
            bootstrap_delivery: BootstrapDelivery::NotRequired,
            capabilities: Some(Vec::new()),
            fault_reap: None,
        }),
        agent: None,
        completed_attachment: None,
    }
}

fn host_status() -> HostStatus {
    HostStatus {
        protocol_version: crate::model::PROTOCOL_VERSION,
        boot_id: TEST_BOOT_ID.into(),
        host_pid: TEST_HOST_PROCESS_ID,
        controller_generation: TEST_HOST_CONTROLLER_GENERATION,
        home: PathBuf::from(TEST_HOST_HOME),
        capabilities: Vec::new(),
    }
}

fn terminal_snapshot(scenario: MatrixScenario) -> TerminalSnapshot {
    let runtime_epoch = match scenario {
        MatrixScenario::MismatchedSnapshot => Uuid::from_u128(MISMATCHED_RUNTIME_EPOCH_VALUE),
        MatrixScenario::Standard
        | MatrixScenario::RuntimeUnavailable
        | MatrixScenario::IncompleteCatalogInventory
        | MatrixScenario::LegacyResumeHost => test_runtime_epoch(),
    };
    TerminalSnapshot {
        runtime_epoch,
        barrier_sequence: TEST_SNAPSHOT_BARRIER_SEQUENCE,
        cols: TEST_SNAPSHOT_COLUMNS,
        rows: TEST_SNAPSHOT_ROWS,
        data: Vec::new(),
    }
}
