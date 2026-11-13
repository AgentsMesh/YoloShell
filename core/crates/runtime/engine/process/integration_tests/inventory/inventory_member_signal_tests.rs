use super::*;
use crate::process::signal_hook::install_pre_signal_hook;
#[path = "../../test_support/inventory/inventory_member_signal.rs"]
mod support;
use support::{MemberProcess, observed};

const IDENTITY_STEP: u64 = 1;
const GROUP_STEP: u32 = 1;
const DELIVERED_ONCE: usize = 1;
const DELIVERY_UNAVAILABLE: &str = "retained exact signal authority unavailable";

#[test]
fn kernel_sigint_is_observed_only_by_the_requested_captured_member() {
    let target_directory = tempfile::tempdir().unwrap();
    let other_directory = tempfile::tempdir().unwrap();
    let target = MemberProcess::observe(target_directory.path());
    let other = MemberProcess::observe(other_directory.path());
    let group = target.group(vec![other.live.clone(), target.live.clone()]);
    assert_eq!(process_group(other.identity.pid).unwrap(), Some(group.id));
    let (count, _guard) = observed();
    assert!(signal_process_group_member(group, target.identity, libc::SIGINT).unwrap());
    support::observer::wait_received(target_directory.path());
    assert_eq!(count.get(), DELIVERED_ONCE);
    assert!(!support::observer::received(other_directory.path()));
    assert!(other.identity.matches());
}

#[test]
fn signal_observer_helper() {
    support::observer::run_if_requested();
}

#[test]
fn exact_member_delivery_does_not_signal_other_captured_group_members() {
    let target = MemberProcess::spawn();
    let other = MemberProcess::spawn();
    let group = target.group(vec![other.live.clone(), target.live.clone()]);
    assert_eq!(process_group(other.identity.pid).unwrap(), Some(group.id));
    let (count, _guard) = observed();
    assert!(signal_process_group_member(group, target.identity, libc::SIGCONT).unwrap());
    assert_eq!(count.get(), DELIVERED_ONCE);
    assert!(other.identity.matches());
}

#[test]
fn absent_target_never_uses_another_live_group_member_as_delivery_evidence() {
    let target = MemberProcess::spawn();
    let other = MemberProcess::spawn();
    let group = target.group(vec![other.live.clone()]);
    let (count, _guard) = observed();
    assert!(!signal_process_group_member(group, target.identity, libc::SIGCONT).unwrap());
    assert_eq!(count.get(), usize::default());
}

#[test]
fn stale_birth_or_replaced_captured_generation_cannot_signal_current_pid() {
    let target = MemberProcess::spawn();
    let stale = ProcessIdentity {
        start_time: target.identity.start_time + IDENTITY_STEP,
        ..target.identity
    };
    let (count, _guard) = observed();
    let group = target.group(vec![target.live.clone()]);
    assert!(!signal_process_group_member(group, stale, libc::SIGCONT).unwrap());
    let replaced = target
        .live
        .with_generation(target.live.lineage_id() + IDENTITY_STEP);
    let group = target.group(vec![replaced]);
    assert!(!signal_process_group_member(group, target.identity, libc::SIGCONT).unwrap());
    assert_eq!(count.get(), usize::default());
}

#[test]
fn captured_member_in_a_different_current_group_is_not_signaled() {
    let target = MemberProcess::spawn();
    let mut group = target.group(vec![target.live.clone()]);
    group.id += GROUP_STEP;
    let (count, _guard) = observed();
    assert!(!signal_process_group_member(group, target.identity, libc::SIGCONT).unwrap());
    assert_eq!(count.get(), usize::default());
}

#[test]
fn target_exit_after_selection_never_falls_back_to_other_member() {
    let target = MemberProcess::spawn();
    let other = MemberProcess::spawn();
    let expected = target.identity;
    let group = target.group(vec![target.live.clone(), other.live.clone()]);
    let _hook = install_pre_signal_hook(move || drop(target));
    let (count, _guard) = observed();
    assert!(!signal_process_group_member(group, expected, libc::SIGCONT).unwrap());
    // The only observed signal is the fixture's exact target cleanup.
    assert_eq!(count.get(), DELIVERED_ONCE);
    assert!(other.identity.matches());
}

#[test]
fn retained_sender_failure_is_not_reported_as_positive_delivery() {
    let target = MemberProcess::spawn();
    let group = target.group(vec![target.live.clone()]);
    let _failure =
        crate::process::install_test_live_signal_failure(|| Some(DELIVERY_UNAVAILABLE.into()));
    let error = signal_process_group_member(group, target.identity, libc::SIGCONT).unwrap_err();
    assert!(error.to_string().contains(DELIVERY_UNAVAILABLE));
}
