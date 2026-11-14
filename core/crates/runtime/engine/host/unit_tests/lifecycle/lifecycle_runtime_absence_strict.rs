use crate::host::tests::*;
#[test]
fn explicit_runtime_absence_is_not_recaptured_as_a_new_runtime() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    host.paths.ensure_session(terminal_id).unwrap();
    let mut selection = TerminalLifecycleSelection::unconditional(terminal_id);
    selection.require_runtime_absent = true;
    assert!(matches!(
        host.capture_lifecycle_mutation_runtime(&selection).unwrap(),
        LifecycleRuntimeFence::Absent
    ));
    selection.expected_runtime_epoch = Some(Uuid::new_v4());
    assert_eq!(
        host.capture_lifecycle_mutation_runtime(&selection)
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
    selection.expected_runtime_epoch = None;
    write_private_bytes(
        &host.paths.runtime_record(terminal_id),
        ABSENT_PENDING_CHECKPOINT_BYTES,
    );
    assert_eq!(
        host.capture_lifecycle_mutation_runtime(&selection)
            .unwrap_err()
            .code,
        crate::protocol::error_code::CONFLICT
    );
}
