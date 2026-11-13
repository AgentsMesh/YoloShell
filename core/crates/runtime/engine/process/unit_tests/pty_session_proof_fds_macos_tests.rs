use super::*;
#[path = "../test_support/pty_descriptor_query_fixture_macos.rs"]
mod fixture;
use fixture::*;

const TEST_PID: u32 = 73;
const TEST_LIMIT: usize = 8;
const TEST_DESCRIPTOR: i32 = 5;
const TEST_FILESYSTEM_DEVICE: u32 = 7;
const TEST_INODE: u64 = 11;
const TEST_CHARACTER_DEVICE: u32 = 13;
const INJECTED_QUERY_ERROR: i32 = libc::EIO;
const ONE_DESCRIPTOR: usize = 1;
const NO_DESCRIPTORS_ALLOWED: usize = 0;
const MISALIGNED_DESCRIPTOR_BYTES: i32 = 1;
const SHORT_RESPONSE_DECREMENT: i32 = 1;

#[test]
fn zero_descriptor_result_distinguishes_empty_from_failure() {
    unsafe { *libc::__error() = libc::EPERM };
    assert!(
        list_descriptors_with(TEST_PID, TEST_LIMIT, empty_query)
            .unwrap()
            .is_empty()
    );
    let error = list_descriptors_with(TEST_PID, TEST_LIMIT, failed_inventory_query).unwrap_err();
    assert_eq!(raw_os_error(&error), Some(INJECTED_QUERY_ERROR));
}

#[test]
fn vnode_query_fails_closed_when_the_descriptor_snapshot_changes() {
    let closed =
        vnode_identity_with(TEST_PID, TEST_DESCRIPTOR, closed_descriptor_query).unwrap_err();
    assert!(matches!(
        &closed,
        CapabilityProofError::CandidateLocalQuery(_)
    ));
    assert_eq!(raw_os_error(&closed), Some(libc::EBADF));
    let invalid = vnode_identity_with(TEST_PID, TEST_DESCRIPTOR, invalid_vnode_query).unwrap_err();
    assert_eq!(raw_os_error(&invalid), Some(libc::EINVAL));
    let exited = vnode_identity_with(TEST_PID, TEST_DESCRIPTOR, exited_process_query).unwrap_err();
    assert_eq!(raw_os_error(&exited), Some(libc::ESRCH));
}

#[test]
fn vnode_query_returns_the_complete_pty_object_identity() {
    assert_ne!(TEST_FILESYSTEM_DEVICE, TEST_CHARACTER_DEVICE);
    let identity = vnode_identity_with(TEST_PID, TEST_DESCRIPTOR, successful_vnode_query).unwrap();
    assert_eq!(
        identity,
        Some(MacosPtyObjectIdentity::from_kernel_fields(
            TEST_FILESYSTEM_DEVICE,
            TEST_INODE,
            TEST_CHARACTER_DEVICE,
        ))
    );
}

#[test]
fn descriptor_inventory_validates_alignment_limit_and_pid_range() {
    let descriptors = list_descriptors_with(TEST_PID, TEST_LIMIT, one_descriptor_query).unwrap();
    assert_eq!(descriptors.len(), ONE_DESCRIPTOR);
    assert_eq!(descriptors[0].proc_fd, TEST_DESCRIPTOR);
    assert_eq!(descriptors[0].proc_fdtype, libc::PROX_FDTYPE_VNODE as u32);
    assert!(matches!(
        list_descriptors_with(TEST_PID, TEST_LIMIT, misaligned_query),
        Err(CapabilityProofError::Invariant(_))
    ));
    assert!(matches!(
        list_descriptors_with(TEST_PID, NO_DESCRIPTORS_ALLOWED, one_descriptor_query),
        Err(CapabilityProofError::ScanBudgetExceeded(_))
    ));
    assert!(matches!(
        list_descriptors_with(u32::MAX, TEST_LIMIT, empty_query),
        Err(CapabilityProofError::Invariant(_))
    ));
}

#[test]
fn short_vnode_response_is_an_invariant_failure() {
    assert!(matches!(
        vnode_identity_with(TEST_PID, TEST_DESCRIPTOR, short_vnode_query),
        Err(CapabilityProofError::Invariant(_))
    ));
}

#[test]
fn descriptor_buffer_uses_only_complete_initialized_entries_within_the_limit() {
    let descriptors = list_descriptors_with(TEST_PID, TEST_LIMIT, exactly_at_limit_query).unwrap();
    assert_eq!(descriptors.len(), TEST_LIMIT);
    assert!(
        descriptors
            .iter()
            .all(|entry| entry.proc_fd == TEST_DESCRIPTOR)
    );
    assert!(
        descriptors
            .iter()
            .all(|entry| entry.proc_fdtype == libc::PROX_FDTYPE_VNODE as u32)
    );
    for query in [
        sentinel_query as ProcessInfoQuery,
        oversized_inventory_query,
    ] {
        assert!(matches!(
            list_descriptors_with(TEST_PID, TEST_LIMIT, query),
            Err(CapabilityProofError::ScanBudgetExceeded(_))
        ));
    }
}

fn raw_os_error(error: &CapabilityProofError) -> Option<i32> {
    let mut cause: &(dyn std::error::Error + 'static) = error;
    loop {
        if let Some(error) = cause.downcast_ref::<std::io::Error>() {
            return error.raw_os_error();
        }
        cause = cause.source()?;
    }
}
