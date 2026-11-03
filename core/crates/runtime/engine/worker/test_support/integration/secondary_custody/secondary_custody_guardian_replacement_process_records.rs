use crate::model::{BootstrapDelivery, RecoveryState, RuntimeRecord, TerminalManifest};
use crate::paths::Paths;
use crate::process::ProcessIdentity;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyDurableRecord, CustodyGuardianBinding, CustodyHolderIdentity,
    CustodyLeaseId, CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};
use crate::sha256_digest::Sha256Digest;
use uuid::Uuid;

pub(super) const AUTH_TOKEN: &str = "guardian-replacement-process-token";
const RUNTIME_SOCKET_NAME: &str = "guardian-replacement-process.sock";
const RUNTIME_COMMAND: &str = "/bin/sleep";
const RUNTIME_DURATION_ARGUMENT: &str = "30";
const OLD_GENERATION: u64 = 17;
pub(super) const CANDIDATE_A_GENERATION: u64 = 18;
pub(super) const CANDIDATE_B_GENERATION: u64 = 19;

pub(super) fn runtime(
    paths: &Paths,
    worker: ProcessIdentity,
    child: ProcessIdentity,
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
        boot_id: crate::process::boot_id(),
        socket_path: paths.home().join(RUNTIME_SOCKET_NAME),
        auth_token: AUTH_TOKEN.to_owned(),
        controller_generation: OLD_GENERATION,
        started_at_ms: crate::model::now_ms(),
        command: RUNTIME_COMMAND.to_owned(),
        args: vec![RUNTIME_DURATION_ARGUMENT.to_owned()],
        cwd: paths.home().to_owned(),
        cols: crate::model::DEFAULT_COLS,
        rows: crate::model::DEFAULT_ROWS,
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
    descriptors: CustodyDescriptorSet,
    digest: Sha256Digest,
    old_host: CustodyHolderIdentity,
    old_guardian: CustodyHolderIdentity,
) -> CustodyDurableRecord {
    let request = request(runtime, old_host, OLD_GENERATION);
    let offer = crate::pty_custody::test_support::custody_offer(&request, descriptors, digest);
    let binding = CustodyGuardianBinding::new(Uuid::new_v4(), old_guardian, &request, &offer)
        .expect("old guardian binding must be exact");
    CustodyDurableRecord::pending(request.runtime.clone())
        .prepare_guardian(&request, &offer, binding)
        .and_then(|prepared| prepared.arm_prepared_guardian(&request, &offer, binding))
        .expect("old guardian lease must be armed")
}

pub(super) fn successor(
    runtime: &RuntimeRecord,
    holder: CustodyHolderIdentity,
    descriptors: CustodyDescriptorSet,
    digest: Sha256Digest,
) -> (CustodyRequest, CustodyOffer) {
    successor_at(runtime, holder, descriptors, digest, CANDIDATE_A_GENERATION)
}

pub(super) fn successor_at(
    runtime: &RuntimeRecord,
    holder: CustodyHolderIdentity,
    descriptors: CustodyDescriptorSet,
    digest: Sha256Digest,
    generation: u64,
) -> (CustodyRequest, CustodyOffer) {
    let request = request(runtime, holder, generation);
    let offer = crate::pty_custody::test_support::custody_offer(&request, descriptors, digest);
    (request, offer)
}

fn request(
    runtime: &RuntimeRecord,
    holder: CustodyHolderIdentity,
    generation: u64,
) -> CustodyRequest {
    CustodyRequest::new(
        CustodyLeaseId::new(),
        CustodyRuntimeIdentity::from_record(runtime),
        holder,
        AUTH_TOKEN.to_owned(),
        generation,
    )
}
