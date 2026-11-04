use crate::model::{BootstrapDelivery, RecoveryState, RuntimeRecord, TerminalManifest};
use crate::paths::Paths;
use crate::process::{ProcessIdentityRelation, ProcessIdentityRelationGuard};
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};
use uuid::Uuid;

pub(super) const AUTH_TOKEN: &str = "guardian-replacement-token";
pub(super) const CHECKPOINT_CHILD_PID: u32 = 71_003;
pub(super) const CHECKPOINT_CHILD_START: u64 = 81_003;
const BOOT_ID: &str = "guardian-replacement-boot";
const SOCKET_NAME: &str = "guardian-replacement.sock";
const SHELL_COMMAND: &str = "/bin/sh";
const OLD_GENERATION: u64 = 7;
pub(super) const NEW_GENERATION: u64 = 8;
pub(super) const OLD_HOST_PID: u32 = 71_001;
pub(super) const OLD_HOST_START: u64 = 81_001;
pub(super) const OLD_GUARDIAN_PID: u32 = 71_002;
pub(super) const OLD_GUARDIAN_START: u64 = 81_002;
const READER_HANDLE: u64 = 91_001;
const WRITER_HANDLE: u64 = 91_002;
const DIGEST_BYTE: u8 = 0x72;
const STARTED_AT_MS: i64 = 1;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;

pub(super) fn runtime(
    paths: &Paths,
    worker: crate::process::ProcessIdentity,
    child: crate::process::ProcessIdentity,
) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: worker.pid,
        worker_start_time: worker.start_time,
        child_pid: child.pid,
        child_start_time: child.start_time,
        boot_id: BOOT_ID.to_owned(),
        socket_path: paths.home().join(SOCKET_NAME),
        auth_token: AUTH_TOKEN.to_owned(),
        controller_generation: OLD_GENERATION,
        started_at_ms: STARTED_AT_MS,
        command: SHELL_COMMAND.to_owned(),
        args: Vec::new(),
        cwd: paths.home().to_owned(),
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::Pending,
    }
}

pub(super) fn manifest(runtime: &RuntimeRecord) -> TerminalManifest {
    TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        name: runtime.command.clone(),
        command: runtime.command.clone(),
        args: runtime.args.clone(),
        cwd: runtime.cwd.clone(),
        cols: runtime.cols,
        rows: runtime.rows,
        created_at_ms: runtime.started_at_ms,
        updated_at_ms: runtime.started_at_ms,
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}

pub(super) fn armed(
    runtime: &RuntimeRecord,
    old_host: CustodyHolderIdentity,
    old_guardian: CustodyHolderIdentity,
) -> CustodyDurableRecord {
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        CustodyRuntimeIdentity::from_record(runtime),
        old_host,
        AUTH_TOKEN.to_owned(),
        OLD_GENERATION,
    );
    let offer = successor_offer(&request);
    let binding =
        CustodyGuardianBinding::new(Uuid::new_v4(), old_guardian, &request, &offer).unwrap();
    CustodyDurableRecord::pending(request.runtime.clone())
        .prepare_guardian(&request, &offer, binding)
        .unwrap()
        .arm_prepared_guardian(&request, &offer, binding)
        .unwrap()
}

pub(super) fn successor_request(
    runtime: &RuntimeRecord,
    process: crate::process::ProcessIdentity,
) -> CustodyRequest {
    CustodyRequest::new(
        CustodyLeaseId::new(),
        CustodyRuntimeIdentity::from_record(runtime),
        holder(process.pid, process.start_time),
        AUTH_TOKEN.to_owned(),
        NEW_GENERATION,
    )
}

pub(super) fn successor_offer(request: &CustodyRequest) -> CustodyOffer {
    CustodyOffer::new_with_profile(
        request,
        CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
        crate::pty_custody::CustodyProfile::ActiveRunning,
        crate::sha256_digest::Sha256Digest::repeated(DIGEST_BYTE),
    )
    .unwrap()
}

pub(super) fn holder(pid: u32, start_time: u64) -> CustodyHolderIdentity {
    CustodyHolderIdentity { pid, start_time }
}

pub(super) fn relation(
    holder: CustodyHolderIdentity,
    relation: ProcessIdentityRelation,
) -> ProcessIdentityRelationGuard {
    crate::process::install_test_process_identity_relation(holder.pid, holder.start_time, relation)
}
