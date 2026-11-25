use super::*;
use yolo_process_test_support::ProcessIdentityObservation;

pub(crate) fn assert_runtime_identities_gone(record: &RuntimeRecord) {
    assert_eq!(
        worker_identity(record).observe(),
        ProcessIdentityObservation::Gone
    );
    assert_eq!(
        child_identity(record).observe(),
        ProcessIdentityObservation::Gone
    );
}

pub(crate) fn assert_runtime_identity_rotated(
    previous: &RuntimeRecord,
    current: &yolo_runtime::model::RuntimeView,
) {
    assert_ne!(current.runtime_epoch, previous.runtime_epoch);
    assert_ne!(current.worker_id, previous.worker_id);
    assert_process_identity_rotated(
        worker_identity(previous),
        ProcessIdentity {
            pid: current.worker_pid,
            start_time: current.worker_start_time,
        },
    );
    assert_process_identity_rotated(
        child_identity(previous),
        ProcessIdentity {
            pid: current.child_pid,
            start_time: current.child_start_time,
        },
    );
}

pub(crate) fn assert_process_identity_rotated(previous: ProcessIdentity, current: ProcessIdentity) {
    assert_ne!(current, previous);
    assert_eq!(current.observe(), ProcessIdentityObservation::Matching);
}

fn worker_identity(record: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: record.worker_pid,
        start_time: record.worker_start_time,
    }
}

fn child_identity(record: &RuntimeRecord) -> ProcessIdentity {
    ProcessIdentity {
        pid: record.child_pid,
        start_time: record.child_start_time,
    }
}
