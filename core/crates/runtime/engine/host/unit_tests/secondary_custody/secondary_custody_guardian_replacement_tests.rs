use super::*;
use crate::process::ProcessIdentityRelation;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyHolderIdentity, CustodyLeaseId, CustodyOffer, CustodyRequest,
    CustodyRuntimeIdentity,
};
use uuid::Uuid;

const AUTH_TOKEN: &str = "guardian-route-token";
const BOOT_ID: &str = "guardian-route-boot";
const GENERATION: u64 = 3;
const WORKER_PID: u32 = 72_001;
const WORKER_START: u64 = 82_001;
const CHILD_PID: u32 = 72_002;
const CHILD_START: u64 = 82_002;
const HOST_PID: u32 = 72_003;
const HOST_START: u64 = 82_003;
const GUARDIAN_PID: u32 = 72_004;
const GUARDIAN_START: u64 = 82_004;
const READER_HANDLE: u64 = 92_001;
const WRITER_HANDLE: u64 = 92_002;
const DIGEST_BYTE: u8 = 0x73;

#[test]
fn indeterminate_guardian_is_rejected_before_spawn_dispatch() {
    let (binding, guardian) = binding();
    let _relation = crate::process::install_test_process_identity_relation(
        guardian.pid,
        guardian.start_time,
        ProcessIdentityRelation::Unknown,
    );

    let error = guardian_route(binding).unwrap_err();
    assert!(matches!(
        error,
        WorkerRpcError::IdentityMismatch(detail) if detail == GUARDIAN_INDETERMINATE_DETAIL
    ));
}

#[test]
fn proven_gone_guardian_selects_replacement_dispatch() {
    let (binding, guardian) = binding();
    let _relation = crate::process::install_test_process_identity_relation(
        guardian.pid,
        guardian.start_time,
        ProcessIdentityRelation::Gone,
    );

    assert_eq!(guardian_route(binding).unwrap(), GuardianRoute::Replacement);
}

fn binding() -> (CustodyGuardianBinding, CustodyHolderIdentity) {
    let runtime = CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START,
        boot_id: BOOT_ID.to_owned(),
    };
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime,
        CustodyHolderIdentity {
            pid: HOST_PID,
            start_time: HOST_START,
        },
        AUTH_TOKEN.to_owned(),
        GENERATION,
    );
    let offer = crate::host::test_custody_offer(
        &request,
        CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
        crate::sha256_digest::Sha256Digest::repeated(DIGEST_BYTE),
    );
    let guardian = CustodyHolderIdentity {
        pid: GUARDIAN_PID,
        start_time: GUARDIAN_START,
    };
    (
        CustodyGuardianBinding::new(Uuid::new_v4(), guardian, &request, &offer).unwrap(),
        guardian,
    )
}
