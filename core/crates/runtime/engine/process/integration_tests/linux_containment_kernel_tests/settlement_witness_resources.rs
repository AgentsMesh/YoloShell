use super::super::{LinuxContainmentError, LinuxContainmentSettlement, descriptor};
use super::custody_fixture::prepare_bound_lease;
use super::record::DurableRecord;
use super::settlement_witness_fixture::{observe, settle};
use super::support::{assert_unfrozen, leaf_path, wait_empty};
use crate::process::ProcessIdentity;
use std::os::fd::AsRawFd;

#[path = "../../test_support/linux/linux_settlement_resources.rs"]
mod resources;
use resources::*;

#[tokio::test]
async fn descriptor_exhaustion_cannot_publish_settlement_or_consume_retained_authority() {
    run_helper().await;
}

#[test]
fn descriptor_pressure_helper() {
    if std::env::var(ROLE_ENVIRONMENT).as_deref() != Ok(HELPER_ROLE) {
        return;
    }
    let (lease, mut child, _pty) = prepare_bound_lease();
    let record = lease.record().clone();
    let durable = DurableRecord::write(&record);
    let root = child.identity();
    let path = leaf_path(&record);
    child.terminate();
    wait_empty(&path);
    let (settled, retired_record) = settle();
    let retired = settled.record().clone();
    let retired_root = descriptor::open_root(&retired.delegated_root).unwrap();
    let retired_child = ProcessIdentity {
        pid: retired.bound_pid.unwrap(),
        start_time: retired.bound_start_time.unwrap(),
    };

    let occupied_lowest = retired_root.try_clone().unwrap();
    let pressure = DescriptorPressure::enter(occupied_lowest.as_raw_fd()).unwrap();
    let capture_present =
        LinuxContainmentSettlement::capture(&lease.root, Some(&lease.leaf), record.clone(), root);
    let capture_absent =
        LinuxContainmentSettlement::capture(&retired_root, None, retired.clone(), retired_child);
    let observe_absent = observe(&retired);
    let verify_retained = settled.verify();
    drop(pressure);
    drop(occupied_lowest);

    assert_emfile("capture retained leaf with zero slots", capture_present);
    assert_emfile("capture absence with zero slots", capture_absent);
    // Boot/descriptor identity reads can fail before opening the delegated root.
    assert_rejected("observe absence with zero slots", observe_absent);
    assert_rejected("verify retained witness with zero slots", verify_retained);

    let lowest_free = retired_root.try_clone().unwrap().as_raw_fd();
    let pressure = DescriptorPressure::one_free_slot(lowest_free).unwrap();
    let partial =
        LinuxContainmentSettlement::capture(&lease.root, Some(&lease.leaf), record.clone(), root);
    let released_partial_slot = retired_root.try_clone().map(|file| file.as_raw_fd());
    let observed_partial = observe(&retired);
    let released_observer_slot = retired_root.try_clone().map(|file| file.as_raw_fd());
    drop(pressure);

    assert_emfile("capture leaf before events with one slot", partial);
    assert_emfile("observe absence with one slot", observed_partial);
    assert_eq!(released_partial_slot.unwrap(), lowest_free);
    assert_eq!(released_observer_slot.unwrap(), lowest_free);
    assert_unfrozen(&path);
    assert!(path.is_dir());
    durable.assert_unchanged();
    retired_record.assert_unchanged();
    settled.verify().unwrap();
    observe(&retired).unwrap().unwrap().verify().unwrap();
    let candidate =
        LinuxContainmentSettlement::capture(&lease.root, Some(&lease.leaf), record, root).unwrap();
    lease.remove_empty().unwrap();
    candidate.verify().unwrap();
    std::fs::File::create(std::env::var_os(COMPLETION_ENVIRONMENT).unwrap()).unwrap();
}

fn assert_emfile<T>(operation: &str, result: Result<T, LinuxContainmentError>) {
    match result {
        Err(LinuxContainmentError::Io(error)) if error.raw_os_error() == Some(libc::EMFILE) => {}
        Err(error) => panic!("{operation}: expected EMFILE, got {error}"),
        Ok(_) => panic!("{operation}: descriptor exhaustion must not authorize settlement"),
    }
}

fn assert_rejected<T>(operation: &str, result: Result<T, LinuxContainmentError>) {
    match result {
        Err(error) => eprintln!("{operation}: {error}"),
        Ok(_) => panic!("{operation}: unavailable evidence must not authorize settlement"),
    }
}
