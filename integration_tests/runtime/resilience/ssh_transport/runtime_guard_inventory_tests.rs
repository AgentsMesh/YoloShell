use super::*;
use crate::support::ChildGuard;
use std::process::Command;
use yolo_runtime::model::{BootstrapDelivery, RecoveryState, RuntimeRecord, RuntimeView};

#[path = "runtime_guard_inventory_fixture.rs"]
mod fixture_support;
use fixture_support::*;

const TEST_BOOT_ID: &str = "detached-inventory-test-boot";
const TEST_CONTROLLER_GENERATION: u64 = 11;
const TEST_COMMAND: &str = "/bin/sleep";
const TEST_ARGUMENT: &str = "60";
const TEST_AUTH_TOKEN: &str = "detached-inventory-test-token";
const TEST_SESSION_NAME: &str = "detached-inventory-test-session";
const FORGED_START_TIME_OFFSET: u64 = 1;
const NO_RUNTIME_AUTHORITY: usize = 0;
const SINGLE_RUNTIME_COUNT: usize = 1;
const BATCH_RUNTIME_COUNT: usize = 2;

#[test]
fn missing_authenticated_inventory_cannot_authorize_disk_candidate() {
    let (mut host, mut victim, mut guard, _) = fixture();

    assert!(guard.bind_authenticated_inventory(&[]).is_err());
    assert_eq!(
        guard.cleanup_authority_counts_for_test(),
        (
            NO_RUNTIME_AUTHORITY,
            SINGLE_RUNTIME_COUNT,
            NO_RUNTIME_AUTHORITY
        )
    );

    settle(&mut host, &mut victim, &mut guard);
}

#[test]
fn forged_inventory_identity_cannot_authorize_disk_candidate() {
    let (mut host, mut victim, mut guard, mut view) = fixture();
    view.runtime.as_mut().unwrap().child_start_time += FORGED_START_TIME_OFFSET;

    assert!(guard.bind_authenticated_inventory(&[view]).is_err());
    assert_eq!(
        guard.cleanup_authority_counts_for_test(),
        (
            NO_RUNTIME_AUTHORITY,
            SINGLE_RUNTIME_COUNT,
            NO_RUNTIME_AUTHORITY
        )
    );

    settle(&mut host, &mut victim, &mut guard);
}

#[test]
fn later_inventory_failure_commits_no_earlier_runtime_authority() {
    let (mut host, mut victims, mut guard, mut views) = inventory_fixture(BATCH_RUNTIME_COUNT);
    views
        .last_mut()
        .unwrap()
        .runtime
        .as_mut()
        .unwrap()
        .child_start_time += FORGED_START_TIME_OFFSET;

    assert!(guard.bind_authenticated_inventory(&views).is_err());
    assert_eq!(
        guard.cleanup_authority_counts_for_test(),
        (
            NO_RUNTIME_AUTHORITY,
            BATCH_RUNTIME_COUNT,
            NO_RUNTIME_AUTHORITY
        )
    );

    guard.disarm_for_test();
    settle_all(&mut victims);
    settle_child(&mut host);
}
