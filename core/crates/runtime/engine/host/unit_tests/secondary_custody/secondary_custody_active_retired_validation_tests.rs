#[cfg(target_os = "macos")]
use super::*;
#[cfg(target_os = "macos")]
use crate::pty_custody::{
    CustodyDurableRecord, CustodyHolderIdentity, CustodyLeaseId, CustodyOffer, CustodyRequest,
    CustodyRuntimeIdentity,
};
#[cfg(target_os = "macos")]
use crate::sha256_digest::Sha256Digest;
#[cfg(target_os = "macos")]
use tempfile::tempdir;

#[cfg(target_os = "macos")]
const RETIRED_PREDECESSOR_GENERATION: u64 = 41;
#[cfg(target_os = "macos")]
const SUCCESSOR_GENERATION_INCREMENT: u64 = 1;
#[cfg(target_os = "macos")]
const RETIRED_ACTIVE_SOCKET_NAME: &str = "retired-active.sock";
#[cfg(target_os = "macos")]
const RETIRED_ACTIVE_DIGEST_BYTE: u8 = 0x6d;

#[cfg(target_os = "macos")]
#[test]
fn active_offer_rejects_retired_macos_predecessor() {
    let directory = tempdir().unwrap();
    let host = Host::open(super::super::super::HostConfig::new(directory.path())).unwrap();
    let record = super::runtime(&host, directory.path().join(RETIRED_ACTIVE_SOCKET_NAME));
    let runtime = CustodyRuntimeIdentity::from_record(&record);
    let holder = CustodyHolderIdentity::capture().unwrap();
    let predecessor_request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime.clone(),
        holder,
        record.auth_token.clone(),
        RETIRED_PREDECESSOR_GENERATION,
    );
    let predecessor_offer = crate::host::test_custody_offer(
        &predecessor_request,
        super::platform_descriptor_set(),
        Sha256Digest::repeated(RETIRED_ACTIVE_DIGEST_BYTE),
    );
    let armed = crate::host::test_arm_custody(
        &CustodyDurableRecord::pending(runtime.clone()),
        &predecessor_request,
        &predecessor_offer,
    )
    .unwrap();
    let retired = crate::pty_custody::test_support::retire_without_evidence(
        &armed,
        armed.armed_lease().unwrap(),
    )
    .unwrap();
    let successor_request = CustodyRequest::new(
        CustodyLeaseId::new(),
        runtime,
        holder,
        record.auth_token,
        RETIRED_PREDECESSOR_GENERATION + SUCCESSOR_GENERATION_INCREMENT,
    );
    let successor_offer = crate::host::test_custody_offer(
        &successor_request,
        super::platform_descriptor_set(),
        Sha256Digest::repeated(RETIRED_ACTIVE_DIGEST_BYTE),
    );

    let result =
        super::super::validation::validate_offer(&successor_request, &successor_offer, &retired);

    assert!(matches!(result, Err(WorkerRpcError::IdentityMismatch(_))));
}
