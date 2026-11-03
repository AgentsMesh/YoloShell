use crate::host::tests::*;
use super::*;

#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_RACE_OUTPUT: &[u8] = b"automatic-exit-race-output";
#[cfg(target_os = "macos")]
pub(super) const AUTOMATIC_EXIT_RACE_SEQUENCE_INCREMENT: u64 = 1;

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_fences_runtime_replacement() {
    let fixture = AutomaticCrashFixture::new();
    fixture.append_output();
    fixture.settle_custody();
    let path = fixture
        .host
        .paths
        .runtime_record(fixture.manifest.terminal_id);
    let mut replacement = fixture.runtime.clone();
    replacement.worker_id = Uuid::new_v4();
    let _guard =
        automatic_worker_crash_exit_test_hook::install(fixture.manifest.terminal_id, move || {
            overwrite_automatic_exit_test_json(&path, &replacement)
        });

    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
    assert!(
        !fixture
            .host
            .paths
            .exit_record(fixture.manifest.terminal_id)
            .exists()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn matching_worker_crash_exit_fences_runtime_replacement() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(
        ExitReason::WorkerCrash,
        fixture.runtime.runtime_epoch,
        sequence,
    );
    let exit_path = fixture.host.paths.exit_record(fixture.manifest.terminal_id);
    let expected_exit: ExitRecord = read_json(&exit_path).unwrap();
    let path = fixture
        .host
        .paths
        .runtime_record(fixture.manifest.terminal_id);
    let mut replacement = fixture.runtime.clone();
    replacement.worker_id = Uuid::new_v4();
    let _guard =
        automatic_worker_crash_exit_test_hook::install(fixture.manifest.terminal_id, move || {
            overwrite_automatic_exit_test_json(&path, &replacement)
        });

    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
    assert_eq!(read_json::<ExitRecord>(&exit_path).unwrap(), expected_exit);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_fences_journal_and_cursor_progress() {
    let fixture = AutomaticCrashFixture::new();
    fixture.append_output();
    fixture.settle_custody();
    let path = fixture.host.paths.journal(fixture.manifest.terminal_id);
    let runtime_epoch = fixture.runtime.runtime_epoch;
    let _guard =
        automatic_worker_crash_exit_test_hook::install(fixture.manifest.terminal_id, move || {
            let mut journal = crate::journal::Journal::open(path, runtime_epoch)?;
            journal.append(AUTOMATIC_EXIT_RACE_OUTPUT)?;
            Ok(())
        });

    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
    assert!(
        !fixture
            .host
            .paths
            .exit_record(fixture.manifest.terminal_id)
            .exists()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_fences_cursor_replacement() {
    let fixture = AutomaticCrashFixture::new();
    fixture.append_output();
    fixture.settle_custody();
    let journal_path = fixture.host.paths.journal(fixture.manifest.terminal_id);
    let cursor_path = crate::journal::retained_cursor_path_for_test(&journal_path);
    let _guard =
        automatic_worker_crash_exit_test_hook::install(fixture.manifest.terminal_id, move || {
            write_private_bytes(&cursor_path, AUTOMATIC_EXIT_RACE_OUTPUT);
            Ok(())
        });

    assert!(
        fixture
            .host
            .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
            .await
            .is_err()
    );
    assert!(
        !fixture
            .host
            .paths
            .exit_record(fixture.manifest.terminal_id)
            .exists()
    );
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_never_overwrites_a_competing_exit() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    let path = fixture.host.paths.exit_record(fixture.manifest.terminal_id);
    let competing = ExitRecord {
        runtime_epoch: fixture.runtime.runtime_epoch,
        code: None,
        signal: None,
        reason: ExitReason::Natural,
        observed_at_ms: now_ms(),
        last_output_sequence: sequence,
    };
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
    let exit: ExitRecord =
        read_json(&fixture.host.paths.exit_record(fixture.manifest.terminal_id)).unwrap();
    assert_eq!(exit.reason, ExitReason::Natural);
}
