use super::*;
use std::cell::RefCell;
use std::collections::VecDeque;

const TARGET_DESCRIPTOR: i32 = 31;
const OTHER_DESCRIPTOR: i32 = 32;
const FILESYSTEM_DEVICE: u32 = 41;
const TARGET_INODE: u64 = 42;
const OTHER_INODE: u64 = 43;
const TERMINAL_DEVICE: u32 = 44;
const NON_VNODE_QUERY_PANIC: &str = "non-vnode descriptor queried as a vnode";

#[test]
fn stable_exact_descriptor_matches_only_its_captured_vnode() {
    let snapshot = vec![descriptor(
        TARGET_DESCRIPTOR,
        libc::PROX_FDTYPE_VNODE as u32,
    )];
    let mut budget = CapabilityScanBudget::start_batch();

    let matched = has_stable_exact_descriptor_with(
        TARGET_DESCRIPTOR,
        identity(TARGET_INODE),
        &mut budget,
        inventories([snapshot.clone(), snapshot]),
        |_| Ok(Some(identity(TARGET_INODE))),
    )
    .unwrap();

    assert!(matched);
}

#[test]
fn stable_absence_and_non_vnode_slots_never_match() {
    let other = vec![descriptor(OTHER_DESCRIPTOR, libc::PROX_FDTYPE_VNODE as u32)];
    let mut absent_budget = CapabilityScanBudget::start_batch();
    assert!(
        !has_stable_exact_descriptor_with(
            TARGET_DESCRIPTOR,
            identity(TARGET_INODE),
            &mut absent_budget,
            inventories([other.clone(), other]),
            |_| Ok(Some(identity(OTHER_INODE))),
        )
        .unwrap()
    );

    let pipe = vec![descriptor(TARGET_DESCRIPTOR, libc::PROX_FDTYPE_PIPE as u32)];
    let mut pipe_budget = CapabilityScanBudget::start_batch();
    assert!(
        !has_stable_exact_descriptor_with(
            TARGET_DESCRIPTOR,
            identity(TARGET_INODE),
            &mut pipe_budget,
            inventories([pipe.clone(), pipe]),
            |_| panic!("{NON_VNODE_QUERY_PANIC}"),
        )
        .unwrap()
    );
}

#[test]
fn descriptor_reuse_between_snapshots_is_indeterminate() {
    let first = vec![descriptor(
        TARGET_DESCRIPTOR,
        libc::PROX_FDTYPE_VNODE as u32,
    )];
    let second = Vec::new();
    let mut budget = CapabilityScanBudget::start_batch();

    let error = has_stable_exact_descriptor_with(
        TARGET_DESCRIPTOR,
        identity(TARGET_INODE),
        &mut budget,
        inventories([first, second]),
        |_| Ok(Some(identity(TARGET_INODE))),
    )
    .unwrap_err();

    assert!(matches!(
        error,
        CapabilityProofError::CandidateLocalQuery(_)
    ));
}

fn inventories<const N: usize>(
    values: [Vec<libc::proc_fdinfo>; N],
) -> impl FnMut() -> CapabilityProofResult<Vec<libc::proc_fdinfo>> {
    let values = RefCell::new(VecDeque::from(values));
    move || Ok(values.borrow_mut().pop_front().unwrap())
}

fn descriptor(number: i32, descriptor_type: u32) -> libc::proc_fdinfo {
    libc::proc_fdinfo {
        proc_fd: number,
        proc_fdtype: descriptor_type,
    }
}

fn identity(inode: u64) -> MacosPtyObjectIdentity {
    MacosPtyObjectIdentity::from_kernel_fields(FILESYSTEM_DEVICE, inode, TERMINAL_DEVICE)
}
