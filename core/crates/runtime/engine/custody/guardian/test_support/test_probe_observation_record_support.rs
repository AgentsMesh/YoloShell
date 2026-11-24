use super::super::*;
use crate::pty_custody::{
    CustodyDescriptorSet, CustodyGuardianBinding, CustodyHolderIdentity, CustodyLeaseId,
    CustodyOffer, CustodyRequest, CustodyRuntimeIdentity,
};

const AUTH_TOKEN: &str = "guardian-set-probe-token";
const BOOT_ID: &str = "guardian-set-probe-boot";
pub(super) const CONTROLLER_GENERATION: u64 = 37;
const WORKER_PID: u32 = 74_001;
const WORKER_START_TIME: u64 = 84_001;
const CHILD_PID: u32 = 74_002;
const CHILD_START_TIME: u64 = 84_002;
const HOST_PID: u32 = 74_003;
const HOST_START_TIME: u64 = 84_003;
const READER_HANDLE: u64 = 94_001;
const WRITER_HANDLE: u64 = 94_002;
const DIGEST_BYTE: u8 = 0x79;

pub(super) fn armed_record(guardian_process: CustodyHolderIdentity) -> CustodyDurableRecord {
    let runtime = CustodyRuntimeIdentity {
        terminal_id: Uuid::new_v4(),
        runtime_epoch: Uuid::new_v4(),
        worker_id: Uuid::new_v4(),
        worker_pid: WORKER_PID,
        worker_start_time: WORKER_START_TIME,
        child_pid: CHILD_PID,
        child_start_time: CHILD_START_TIME,
        boot_id: BOOT_ID.to_owned(),
    };
    let request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        CustodyHolderIdentity {
            pid: HOST_PID,
            start_time: HOST_START_TIME,
        },
        AUTH_TOKEN.to_owned(),
        CONTROLLER_GENERATION,
    );
    let offer = CustodyOffer::new_with_profile(
        &request,
        CustodyDescriptorSet::macos_process_capability(READER_HANDLE, WRITER_HANDLE),
        crate::pty_custody::CustodyProfile::ActiveRunning,
        crate::sha256_digest::Sha256Digest::repeated(DIGEST_BYTE),
    )
    .unwrap();
    let guardian =
        CustodyGuardianBinding::new(Uuid::new_v4(), guardian_process, &request, &offer).unwrap();
    CustodyDurableRecord::pending(runtime)
        .prepare_guardian(&request, &offer, guardian)
        .unwrap()
        .arm_prepared_guardian(&request, &offer, guardian)
        .unwrap()
}
