use crate::model::{BootstrapDelivery, RecoveryState, RuntimeRecord, TerminalManifest};
use crate::paths::Paths;
use crate::process::ProcessIdentity;
use crate::pty_custody::sha256_digest::Sha256Digest;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyProfile, CustodyRequest, CustodyRuntimeIdentity,
};

pub(super) const CONTROLLER_GENERATION: u64 = 7;
const TEST_AUTH_TOKEN: &str = "prepared-retirement-publisher-token";
const SESSION_NAME: &str = "prepared retirement publisher";
const SOCKET_NAME: &str = "prepared-retirement-publisher.sock";
const SHELL_COMMAND: &str = "/bin/sleep";
const SHELL_DURATION: &str = "30";
const STARTED_AT_MS: i64 = 1;
const TERMINAL_COLUMNS: u16 = 80;
const TERMINAL_ROWS: u16 = 24;

pub(super) fn runtime_record(paths: &Paths, runtime: &CustodyRuntimeIdentity) -> RuntimeRecord {
    RuntimeRecord {
        schema_version: crate::constants::RUNTIME_RECORD_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        runtime_epoch: runtime.runtime_epoch,
        worker_id: runtime.worker_id,
        worker_pid: runtime.worker_pid,
        worker_start_time: runtime.worker_start_time,
        child_pid: runtime.child_pid,
        child_start_time: runtime.child_start_time,
        boot_id: runtime.boot_id.clone(),
        socket_path: paths.home().join(SOCKET_NAME),
        auth_token: TEST_AUTH_TOKEN.to_owned(),
        controller_generation: CONTROLLER_GENERATION,
        started_at_ms: STARTED_AT_MS,
        command: SHELL_COMMAND.to_owned(),
        args: vec![SHELL_DURATION.to_owned()],
        cwd: paths.home().to_owned(),
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        bootstrap_delivery: BootstrapDelivery::Delivered,
    }
}

pub(super) fn manifest(paths: &Paths, runtime: &CustodyRuntimeIdentity) -> TerminalManifest {
    TerminalManifest {
        schema_version: crate::constants::SESSION_MANIFEST_SCHEMA_VERSION,
        terminal_id: runtime.terminal_id,
        name: SESSION_NAME.to_owned(),
        command: SHELL_COMMAND.to_owned(),
        args: vec![SHELL_DURATION.to_owned()],
        cwd: paths.home().to_owned(),
        cols: TERMINAL_COLUMNS,
        rows: TERMINAL_ROWS,
        created_at_ms: STARTED_AT_MS,
        updated_at_ms: STARTED_AT_MS,
        archived: false,
        recovery_state: RecoveryState::Created,
    }
}

pub(super) fn prepared(
    runtime: &CustodyRuntimeIdentity,
    descriptor_set: CustodyDescriptorSet,
    digest: Sha256Digest,
) -> CustodyDurableRecord {
    let process = ProcessIdentity::capture(std::process::id()).unwrap();
    let holder = CustodyHolderIdentity {
        pid: process.pid,
        start_time: process.start_time,
    };
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        holder,
        TEST_AUTH_TOKEN.to_owned(),
        CONTROLLER_GENERATION,
    );
    let offer = CustodyOffer::new_with_profile(
        &request,
        descriptor_set,
        CustodyProfile::ActiveRunning,
        digest,
    )
    .unwrap();
    let binding =
        CustodyGuardianBinding::new(uuid::Uuid::new_v4(), holder, &request, &offer).unwrap();
    CustodyDurableRecord::pending(runtime.clone())
        .prepare_guardian(&request, &offer, binding)
        .unwrap()
}
