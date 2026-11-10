use crate::host::tests::*;
#[cfg(target_os = "macos")]
use super::automatic_worker_crash_exit_races::AUTOMATIC_EXIT_RACE_SEQUENCE_INCREMENT;
use super::*;

#[cfg(target_os = "macos")]
#[tokio::test]
async fn matching_worker_crash_exit_fences_exit_replacement() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(
        ExitReason::WorkerCrash,
        fixture.runtime.runtime_epoch,
        sequence,
    );
    let path = fixture.host.paths.exit_record(fixture.manifest.terminal_id);
    let competing = ExitRecord {
        runtime_epoch: fixture.runtime.runtime_epoch,
        code: None,
        signal: None,
        reason: ExitReason::WorkerCrash,
        observed_at_ms: now_ms(),
        last_output_sequence: sequence + AUTOMATIC_EXIT_RACE_SEQUENCE_INCREMENT,
    };
    let expected = competing.clone();
    let _guard =
        automatic_worker_crash_exit_test_hook::install(fixture.manifest.terminal_id, move || {
            overwrite_automatic_exit_test_json(&path, &competing)
        });

    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
    let installed: ExitRecord =
        read_json(&fixture.host.paths.exit_record(fixture.manifest.terminal_id)).unwrap();
    assert_eq!(installed, expected);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn matching_worker_crash_exit_retries_parent_sync_without_replacement() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(
        ExitReason::WorkerCrash,
        fixture.runtime.runtime_epoch,
        sequence,
    );
    let path = fixture.host.paths.exit_record(fixture.manifest.terminal_id);
    let before: (ExitRecord, crate::paths::identity::FileIdentity) = fixture
        .host
        .namespace
        .read_json_with_identity(&path)
        .unwrap();

    crate::paths::test_support::fail_next_directory_sync();
    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
    let ambiguous: (ExitRecord, crate::paths::identity::FileIdentity) = fixture
        .host
        .namespace
        .read_json_with_identity(&path)
        .unwrap();
    assert_eq!(ambiguous, before);

    fixture
        .host
        .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
        .await
        .unwrap();
    let confirmed: (ExitRecord, crate::paths::identity::FileIdentity) = fixture
        .host
        .namespace
        .read_json_with_identity(&path)
        .unwrap();
    assert_eq!(confirmed, before);
}
