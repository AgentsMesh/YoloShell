use yolo_paths::{Paths, namespace_lease::NamespaceLease};
use yolo_terminal_model::{ExitRecord, RuntimeRecord, TerminationTombstone};

pub fn exit_or_tombstone_matches(
    paths: &Paths,
    namespace: &NamespaceLease,
    record: &RuntimeRecord,
) -> bool {
    namespace
        .read_json::<ExitRecord>(&paths.exit_record(record.terminal_id))
        .is_ok_and(|exit| exit.runtime_epoch == record.runtime_epoch)
        || stable_termination_tombstone_matches(paths, namespace, record)
}

pub fn stable_termination_tombstone_matches(
    paths: &Paths,
    namespace: &NamespaceLease,
    expected_runtime: &RuntimeRecord,
) -> bool {
    let runtime_path = paths.runtime_record(expected_runtime.terminal_id);
    let Ok((runtime, runtime_identity)) =
        namespace.read_json_with_identity::<RuntimeRecord>(&runtime_path)
    else {
        return false;
    };
    if runtime != *expected_runtime {
        return false;
    }
    let tombstone_path = paths.terminate_tombstone(expected_runtime.terminal_id);
    let Ok((tombstone, tombstone_identity)) =
        namespace.read_json_with_identity::<TerminationTombstone>(&tombstone_path)
    else {
        return false;
    };
    if tombstone
        .validate_for(runtime.terminal_id, runtime.runtime_epoch)
        .is_err()
    {
        return false;
    }
    let tombstone_stable = namespace
        .read_json_with_identity::<TerminationTombstone>(&tombstone_path)
        .is_ok_and(|current| current == (tombstone, tombstone_identity));
    let runtime_stable = namespace
        .read_json_with_identity::<RuntimeRecord>(&runtime_path)
        .is_ok_and(|current| current == (runtime, runtime_identity));
    tombstone_stable && runtime_stable
}
