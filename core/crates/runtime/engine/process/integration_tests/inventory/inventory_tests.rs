use super::inventory::{
    ProcessGroupIdentity, PtyProcessScope, process_group, signal_process_group,
};
use super::inventory_snapshot::{InventoryEntry, is_descendant, parent_map, parse_inventory_line};
use super::inventory_test_support::TestProcess;
use super::*;
use std::collections::HashSet;

const ROOT_PID: u32 = 10;
const CHILD_PID: u32 = 11;
const GRANDCHILD_PID: u32 = 12;
const STALE_GENERATION_OFFSET: u64 = 1;

#[test]
fn inventory_parser_and_descendant_walk_are_bounded() {
    let pid = std::process::id();
    let parent_pid = unsafe { libc::getppid() } as u32;
    let process_group = unsafe { libc::getpgrp() } as u32;
    #[cfg(target_os = "linux")]
    let suffix = format!(
        " {effective_user_id}",
        effective_user_id = unsafe { libc::geteuid() }
    );
    #[cfg(not(target_os = "linux"))]
    let suffix = String::new();
    let parsed =
        parse_inventory_line(&format!("{pid} {parent_pid} {process_group}{suffix}")).unwrap();
    assert_eq!(parsed.pid, pid);
    assert_eq!(parsed.parent_pid, parent_pid);
    assert_eq!(parsed.process_group, process_group);
    assert!(parsed.session_id > 0);
    assert!(parse_inventory_line("missing fields").is_none());

    let entries = [
        entry(ROOT_PID, ROOT_PID),
        entry(CHILD_PID, ROOT_PID),
        entry(GRANDCHILD_PID, CHILD_PID),
    ];
    let parents = parent_map(&entries);
    assert!(is_descendant(GRANDCHILD_PID, ROOT_PID, &parents));
    assert!(!is_descendant(ROOT_PID, GRANDCHILD_PID, &parents));
}

#[cfg(target_os = "macos")]
#[test]
fn native_inventory_contains_the_current_kernel_identity() {
    let pid = std::process::id();
    let entries = super::inventory_snapshot::inventory().unwrap();
    let current = entries.iter().find(|entry| entry.pid == pid).unwrap();
    assert_eq!(current.parent_pid, unsafe { libc::getppid() } as u32);
    assert_eq!(current.process_group, unsafe { libc::getpgrp() } as u32);
    assert!(current.session_id > 0);
    assert_eq!(current.identity(), ProcessIdentity::capture(pid).unwrap());
}

#[test]
fn live_scope_uses_kernel_identities_and_rejects_unverified_groups() {
    let current = ProcessIdentity::capture(std::process::id()).unwrap();
    let scope = PtyProcessScope::capture(current).unwrap();
    let identities = scope.identities(&Default::default()).unwrap();
    assert!(identities.contains(&current));
    assert!(scope.foreground_group(0).is_err());

    let live = LiveProcessIdentity::capture(current).unwrap().unwrap();
    let stale_group = ProcessGroupIdentity::verified(
        current.pid,
        vec![live.with_generation(live.lineage_id().wrapping_add(STALE_GENERATION_OFFSET))],
    );
    assert!(!signal_process_group(stale_group, libc::SIGCONT).unwrap());
}

#[test]
fn live_scope_filters_stale_anchors_and_resolves_the_foreground_group() {
    let current = ProcessIdentity::capture(std::process::id()).unwrap();
    let stale = ProcessIdentity {
        start_time: current.start_time.wrapping_add(1),
        ..current
    };
    assert!(!stale.matches());
    assert!(PtyProcessScope::capture(stale).is_err());
    let scope = PtyProcessScope::capture(current).unwrap();
    assert_eq!(scope.root(), current);
    let anchors = HashSet::from([current, stale]);
    let identities = scope.identities(&anchors).unwrap();
    assert!(identities.contains(&current));

    let target = TestProcess::spawn();
    let target_scope = PtyProcessScope::capture(target.identity).unwrap();
    let group_id = target.identity.pid as i32;
    let group = target_scope.foreground_group(group_id).unwrap();
    assert_eq!(group.id(), target.identity.pid);
    assert!(signal_process_group(group, libc::SIGCONT).unwrap());
    assert!(current.signal(libc::SIGCONT).unwrap());
    assert!(!stale.signal(libc::SIGCONT).unwrap());
}

#[test]
fn process_group_lookup_distinguishes_live_missing_and_out_of_range_pids() {
    let current_group = unsafe { libc::getpgrp() } as u32;
    assert_eq!(
        process_group(std::process::id()).unwrap(),
        Some(current_group)
    );
    assert_eq!(process_group(i32::MAX as u32).unwrap(), None);
    assert!(process_group(u32::MAX).is_err());
    assert!(ProcessIdentity::capture(0).is_err());
}

fn entry(pid: u32, parent_pid: u32) -> InventoryEntry {
    InventoryEntry {
        pid,
        parent_pid,
        process_group: ROOT_PID,
        session_id: ROOT_PID,
        start_time: u64::from(pid),
        #[cfg(target_os = "linux")]
        effective_user_id: unsafe { libc::geteuid() },
        #[cfg(target_os = "macos")]
        unique_id: u64::from(pid),
        #[cfg(target_os = "macos")]
        parent_unique_id: u64::from(parent_pid),
    }
}
