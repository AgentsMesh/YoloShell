use super::{
    CgroupDirectoryIdentity, LinuxContainmentError, LinuxContainmentRecord, custody_constants,
};
use crate::sha256_digest::Sha256Digest;
use serde::Serialize;

const DIGEST_FAILURE_DETAIL: &str = "containment custody identity serialization failed";

#[derive(Serialize)]
struct RecoveryIdentity<'a> {
    domain: &'static str,
    record: &'a LinuxContainmentRecord,
    root_descriptor: CgroupDirectoryIdentity,
    leaf_descriptor: CgroupDirectoryIdentity,
}

pub(super) fn capture(
    record: &LinuxContainmentRecord,
    root_descriptor: CgroupDirectoryIdentity,
    leaf_descriptor: CgroupDirectoryIdentity,
) -> Result<Sha256Digest, LinuxContainmentError> {
    Sha256Digest::capture(&RecoveryIdentity {
        domain: custody_constants::RECOVERY_IDENTITY_DIGEST_DOMAIN,
        record,
        root_descriptor,
        leaf_descriptor,
    })
    .map_err(|_| LinuxContainmentError::Identity(DIGEST_FAILURE_DETAIL))
}
