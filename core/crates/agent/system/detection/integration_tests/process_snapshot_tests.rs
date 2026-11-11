use super::super::ObservedProcess;
use super::test_support::{
    TEST_FAILURE, TEST_ROOT_START_TIME, frame, observed_wire, receipt, root,
};
use super::*;
use std::io::Cursor;
use std::os::unix::ffi::OsStringExt;
use std::path::PathBuf;

#[tokio::test]
async fn stable_snapshot_round_trips_through_private_wire_types() {
    let wire = observed_wire();
    let expected = ObservedProcess::from(observed_wire());
    let mut input = Cursor::new(frame(ProcessSnapshotWireOutcome::Stable {
        processes: vec![wire],
    }));

    let actual = read_process_snapshot(&mut input, root(), super::test_support::environment_keys()).await.unwrap();

    assert_eq!(
        actual,
        receipt(ProcessSnapshotOutcome::Stable(vec![expected]))
    );
}

#[tokio::test]
async fn snapshot_preserves_non_utf8_unix_paths() {
    const NON_UTF8_BYTE: u8 = 0xff;
    let mut process = observed_wire();
    let path = PathBuf::from(std::ffi::OsString::from_vec(vec![
        b'/',
        b't',
        b'm',
        b'p',
        b'/',
        NON_UTF8_BYTE,
    ]));
    process.cwd = Some(path.clone().into());
    let mut input = Cursor::new(frame(ProcessSnapshotWireOutcome::Stable {
        processes: vec![process],
    }));

    let actual = read_process_snapshot(&mut input, root(), super::test_support::environment_keys()).await.unwrap();
    let ProcessSnapshotOutcome::Stable(processes) = actual.outcome else {
        panic!("stable process snapshot expected");
    };
    assert_eq!(processes[0].cwd.as_ref(), Some(&path));
}

#[tokio::test]
async fn non_stable_snapshot_outcomes_round_trip() {
    let mut changed = Cursor::new(frame(ProcessSnapshotWireOutcome::RootChanged));
    assert_eq!(
        read_process_snapshot(&mut changed, root(), super::test_support::environment_keys()).await.unwrap(),
        receipt(ProcessSnapshotOutcome::RootChanged)
    );
    let mut failed = Cursor::new(frame(ProcessSnapshotWireOutcome::Failed {
        detail: TEST_FAILURE.to_owned(),
    }));
    assert_eq!(
        read_process_snapshot(&mut failed, root(), super::test_support::environment_keys()).await.unwrap(),
        receipt(ProcessSnapshotOutcome::Failed(TEST_FAILURE.to_owned()))
    );
}

#[test]
fn snapshot_errors_are_nonempty_utf8_bounded_details() {
    assert_eq!(
        bounded_error(String::new()),
        PROCESS_SNAPSHOT_ERROR_FALLBACK
    );
    let detail = "界".repeat(MAX_PROCESS_SNAPSHOT_ERROR_BYTES);
    let bounded = bounded_error(detail);
    assert!(!bounded.is_empty());
    assert!(bounded.len() <= MAX_PROCESS_SNAPSHOT_ERROR_BYTES);
    assert!(bounded.is_char_boundary(bounded.len()));
}

#[test]
fn system_snapshot_rejects_an_invalid_root_before_observation() {
    for pid in [u32::default(), u32::MAX] {
        let invalid = ProcessIdentity {
            pid,
            start_time: TEST_ROOT_START_TIME,
        };
        assert!(write_system_process_snapshot(invalid, &[], Vec::new()).is_err());
    }
}

#[test]
fn snapshot_writer_emits_valid_processes_as_stable() {
    let process = ObservedProcess::from(observed_wire());

    let outcome = encode::stable(vec![process], &super::test_support::environment_keys());

    assert!(matches!(
        outcome,
        ProcessSnapshotWireOutcome::Stable { processes } if processes.len() == 1
    ));
}

#[test]
fn snapshot_writer_rejects_oversized_argv_before_framing() {
    let mut process = observed_wire();
    process.argv = vec![String::new(); limits::MAX_PROCESS_SNAPSHOT_ARGV_ITEMS + 1];

    let outcome = encode::stable(vec![ObservedProcess::from(process)], &super::test_support::environment_keys());

    assert!(matches!(
        outcome,
        ProcessSnapshotWireOutcome::Failed { detail }
            if !detail.is_empty() && detail.len() <= MAX_PROCESS_SNAPSHOT_ERROR_BYTES
    ));
}
