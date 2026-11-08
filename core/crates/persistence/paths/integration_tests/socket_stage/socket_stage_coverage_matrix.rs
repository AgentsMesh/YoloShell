use super::super::*;
use std::fs;

const INVALID_LEASE_BYTES: &[u8] = b"{";

#[derive(Clone, Copy)]
enum CleanupCase {
    Busy,
    Unproven,
    Mutated,
}

#[test]
fn shared_cleanup_instance_covers_every_lease_state() {
    for case in [
        CleanupCase::Busy,
        CleanupCase::Unproven,
        CleanupCase::Mutated,
    ] {
        run_cleanup(case);
    }
}

#[test]
fn lifecycle_cleanup_distinguishes_absence_unsafe_shape_and_bookkeeping() {
    let (_temporary, directory, target) = super::fixture();
    assert!(matches!(
        cleanup_lifecycle(&directory).unwrap(),
        Cleanup::Unchanged
    ));

    fs::create_dir(target.lifecycle_path()).unwrap();
    assert!(matches!(
        cleanup_lifecycle(&directory).unwrap(),
        Cleanup::Unproven
    ));
    fs::remove_dir(target.lifecycle_path()).unwrap();

    drop(Lease::acquire_immediate_for_test(&directory, &target).unwrap());
    assert!(matches!(
        cleanup_lifecycle(&directory).unwrap(),
        Cleanup::Unproven
    ));
    assert!(matches!(
        cleanup_key(&directory, target.key()).unwrap(),
        Cleanup::Mutated
    ));
    assert!(matches!(
        cleanup_lifecycle(&directory).unwrap(),
        Cleanup::Mutated
    ));
}

fn run_cleanup(case: CleanupCase) {
    let (_temporary, directory, target) = super::fixture();
    let lease = Lease::acquire_immediate_for_test(&directory, &target).unwrap();
    let held = match case {
        CleanupCase::Busy => Some(lease),
        CleanupCase::Unproven | CleanupCase::Mutated => {
            drop(lease);
            None
        }
    };
    if matches!(case, CleanupCase::Unproven) {
        fs::write(target.lease().unwrap().1, INVALID_LEASE_BYTES).unwrap();
    }
    let result = cleanup_key_with_all_test_hooks(
        &directory,
        target.key(),
        directory.path(),
        || Ok(()),
        || Ok(()),
        || Ok(()),
        || Ok(()),
    )
    .unwrap();
    let expected = match case {
        CleanupCase::Busy => matches!(result, Cleanup::Busy),
        CleanupCase::Unproven => matches!(result, Cleanup::Unproven),
        CleanupCase::Mutated => matches!(result, Cleanup::Mutated),
    };
    assert!(expected);
    drop(held);
}
