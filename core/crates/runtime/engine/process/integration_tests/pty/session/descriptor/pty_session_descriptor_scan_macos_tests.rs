use super::*;
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;

const ORIGINAL_DESCRIPTOR: i32 = 31;
const MOVED_DESCRIPTOR: i32 = 32;
const FILESYSTEM_DEVICE: u32 = 51;
const ORIGINAL_INODE: u64 = 52;
const REPLACEMENT_INODE: u64 = 53;
const SECOND_REPLACEMENT_INODE: u64 = 54;
const UNRELATED_RDEV: u32 = 61;
const TARGET_TTY_RDEV: u32 = 62;
const SINGLE_DESCRIPTOR_SNAPSHOT: usize = 1;

#[test]
fn second_pass_finds_a_tty_moved_outside_the_first_snapshot() {
    let inventories = inventories([
        vec![descriptor(ORIGINAL_DESCRIPTOR)],
        vec![
            descriptor(ORIGINAL_DESCRIPTOR),
            descriptor(MOVED_DESCRIPTOR),
        ],
    ]);
    let mut budget = CapabilityScanBudget::start_batch();

    let matched =
        has_stable_tty_descriptor_with(target(), &mut budget, inventories, |descriptor| {
            Ok(Some(if descriptor == MOVED_DESCRIPTOR {
                target()
            } else {
                unrelated(ORIGINAL_INODE)
            }))
        })
        .unwrap();

    assert!(matched);
}

#[test]
fn changed_negative_snapshots_remain_indeterminate() {
    let inventories = inventories([
        vec![descriptor(ORIGINAL_DESCRIPTOR)],
        vec![
            descriptor(ORIGINAL_DESCRIPTOR),
            descriptor(MOVED_DESCRIPTOR),
        ],
    ]);
    let mut budget = CapabilityScanBudget::start_batch();

    let error = has_stable_tty_descriptor_with(target(), &mut budget, inventories, |_descriptor| {
        Ok(Some(unrelated(ORIGINAL_INODE)))
    })
    .err()
    .unwrap();

    assert!(matches!(
        error,
        CapabilityProofError::CandidateLocalQuery(_)
    ));
}

#[test]
fn identical_negative_snapshots_prove_tty_absence() {
    let snapshot = vec![descriptor(ORIGINAL_DESCRIPTOR)];
    let inventories = inventories([snapshot.clone(), snapshot]);
    let mut budget = CapabilityScanBudget::start_batch();

    let matched =
        has_stable_tty_descriptor_with(target(), &mut budget, inventories, |_descriptor| {
            Ok(Some(unrelated(ORIGINAL_INODE)))
        })
        .unwrap();

    assert!(!matched);
}

#[test]
fn inode_change_with_the_same_fd_and_rdev_is_indeterminate() {
    let inventories = inventories([
        vec![descriptor(ORIGINAL_DESCRIPTOR)],
        vec![descriptor(ORIGINAL_DESCRIPTOR)],
    ]);
    let identities = RefCell::new(VecDeque::from([
        unrelated(REPLACEMENT_INODE),
        unrelated(SECOND_REPLACEMENT_INODE),
    ]));
    let mut budget = CapabilityScanBudget::start_batch();

    let error = has_stable_tty_descriptor_with(target(), &mut budget, inventories, |_descriptor| {
        Ok(Some(identities.borrow_mut().pop_front().unwrap()))
    })
    .err()
    .unwrap();

    assert!(matches!(
        error,
        CapabilityProofError::CandidateLocalQuery(_)
    ));
}

#[test]
fn same_rdev_with_a_replaced_inode_never_matches_the_target() {
    let snapshot = vec![descriptor(ORIGINAL_DESCRIPTOR)];
    let inventories = inventories([snapshot.clone(), snapshot]);
    let replacement = MacosPtyObjectIdentity::from_kernel_fields(
        FILESYSTEM_DEVICE,
        REPLACEMENT_INODE,
        TARGET_TTY_RDEV,
    );
    let mut budget = CapabilityScanBudget::start_batch();

    let matched = has_stable_tty_descriptor_with(target(), &mut budget, inventories, |_| {
        Ok(Some(replacement))
    })
    .unwrap();

    assert!(!matched);
}

#[test]
fn negative_observation_reads_one_descriptor_snapshot() {
    let list_calls = Cell::new(usize::default());
    let vnode_calls = Cell::new(usize::default());
    let mut budget = CapabilityScanBudget::start_batch();

    let matched = observes_tty_descriptor_with(
        target(),
        &mut budget,
        || {
            list_calls.set(list_calls.get() + SINGLE_DESCRIPTOR_SNAPSHOT);
            Ok(vec![descriptor(ORIGINAL_DESCRIPTOR)])
        },
        |_descriptor| {
            vnode_calls.set(vnode_calls.get() + SINGLE_DESCRIPTOR_SNAPSHOT);
            Ok(Some(unrelated(ORIGINAL_INODE)))
        },
    )
    .unwrap();

    assert!(!matched);
    assert_eq!(list_calls.get(), SINGLE_DESCRIPTOR_SNAPSHOT);
    assert_eq!(vnode_calls.get(), SINGLE_DESCRIPTOR_SNAPSHOT);
    assert_eq!(budget.descriptor_count(), SINGLE_DESCRIPTOR_SNAPSHOT);
}

fn inventories<const N: usize>(
    values: [Vec<libc::proc_fdinfo>; N],
) -> impl FnMut() -> CapabilityProofResult<Vec<libc::proc_fdinfo>> {
    let values = RefCell::new(VecDeque::from(values));
    move || Ok(values.borrow_mut().pop_front().unwrap())
}

fn descriptor(number: i32) -> libc::proc_fdinfo {
    libc::proc_fdinfo {
        proc_fd: number,
        proc_fdtype: libc::PROX_FDTYPE_VNODE as u32,
    }
}

fn target() -> MacosPtyObjectIdentity {
    MacosPtyObjectIdentity::from_kernel_fields(FILESYSTEM_DEVICE, ORIGINAL_INODE, TARGET_TTY_RDEV)
}

fn unrelated(inode: u64) -> MacosPtyObjectIdentity {
    MacosPtyObjectIdentity::from_kernel_fields(FILESYSTEM_DEVICE, inode, UNRELATED_RDEV)
}
