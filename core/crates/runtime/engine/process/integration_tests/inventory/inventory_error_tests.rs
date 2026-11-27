use super::constants::PROCESS_INVENTORY_CAPTURE_ATTEMPTS;
use super::inventory_snapshot::{
    install_repeating_test_inventory, install_test_inventory_results, inventory,
};
use super::inventory_stable::{INVENTORY_CAPTURE_EXHAUSTED_CONTEXT, StableInventory};
use super::inventory_test_support::{NO_PARENT_PID, TestProcess, entry};
use std::io;

const FIRST_SNAPSHOT_FAILURE_DETAIL: &str = "injected first inventory snapshot failure";
const SECOND_SNAPSHOT_FAILURE_DETAIL: &str = "injected second inventory snapshot failure";
const SECOND_SNAPSHOT_CONSUMED_DETAIL: &str = "first snapshot failure consumed its successor";

#[test]
fn stable_inventory_retries_the_first_complete_snapshot() {
    let _inventory = install_test_inventory_results([
        Err(io::Error::other(FIRST_SNAPSHOT_FAILURE_DETAIL)),
        Ok(Vec::new()),
        Ok(Vec::new()),
    ]);

    assert!(StableInventory::capture().unwrap().entries().is_empty());
}

#[test]
fn stable_inventory_retries_the_second_complete_snapshot() {
    let _inventory = install_test_inventory_results([
        Ok(Vec::new()),
        Err(io::Error::other(SECOND_SNAPSHOT_FAILURE_DETAIL)),
        Ok(Vec::new()),
    ]);

    assert!(StableInventory::capture().unwrap().entries().is_empty());
}

#[test]
fn stable_inventory_propagates_a_persistent_snapshot_failure() {
    let failures = std::iter::repeat_with(|| Err(io::Error::other(FIRST_SNAPSHOT_FAILURE_DETAIL)))
        .take(PROCESS_INVENTORY_CAPTURE_ATTEMPTS)
        .chain([Ok(Vec::new())]);
    let _inventory = install_test_inventory_results(failures);
    let error = StableInventory::capture().err().unwrap();

    assert_eq!(error.to_string(), INVENTORY_CAPTURE_EXHAUSTED_CONTEXT);
    assert!(
        error
            .chain()
            .any(|cause| cause.to_string() == FIRST_SNAPSHOT_FAILURE_DETAIL)
    );
    assert!(
        inventory().unwrap().is_empty(),
        "{SECOND_SNAPSHOT_CONSUMED_DETAIL}"
    );
}

#[test]
fn repeating_inventory_can_be_replaced_without_consumption() {
    let process = TestProcess::spawn();
    let expected = entry(process.identity, NO_PARENT_PID, process.identity.pid);
    let mut inventory_fixture = install_repeating_test_inventory(vec![expected]);

    assert_eq!(inventory().unwrap(), vec![expected]);
    inventory_fixture.replace_repeating(Vec::new());
    assert!(inventory().unwrap().is_empty());
}

#[test]
fn repeating_inventory_discards_an_exited_identity() {
    let mut process = TestProcess::spawn();
    let expected = entry(process.identity, NO_PARENT_PID, process.identity.pid);
    let _inventory_fixture = install_repeating_test_inventory(vec![expected]);

    process.terminate();

    assert!(inventory().unwrap().is_empty());
}
