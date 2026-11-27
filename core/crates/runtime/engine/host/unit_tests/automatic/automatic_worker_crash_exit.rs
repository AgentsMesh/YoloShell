use crate::host::tests::*;
use super::*;

#[cfg(target_os = "macos")]
const AUTOMATIC_EXIT_TEST_STALE_REASON: ExitReason = ExitReason::Natural;
#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_uses_retained_journal_sequence() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();

    fixture
        .host
        .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
        .await
        .unwrap();

    let exit: ExitRecord =
        read_json(&fixture.host.paths.exit_record(fixture.manifest.terminal_id)).unwrap();
    assert_eq!(exit.runtime_epoch, fixture.runtime.runtime_epoch);
    assert_eq!(exit.reason, ExitReason::WorkerCrash);
    assert_eq!(exit.last_output_sequence, sequence);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_replaces_only_missing_or_stale_evidence() {
    let fixture = AutomaticCrashFixture::new();
    fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(
        AUTOMATIC_EXIT_TEST_STALE_REASON,
        Uuid::new_v4(),
        AUTOMATIC_EXIT_TEST_EMPTY_SEQUENCE,
    );

    fixture
        .host
        .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
        .await
        .unwrap();

    let exit: ExitRecord =
        read_json(&fixture.host.paths.exit_record(fixture.manifest.terminal_id)).unwrap();
    assert_eq!(exit.reason, ExitReason::WorkerCrash);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_preserves_matching_worker_crash() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(
        ExitReason::WorkerCrash,
        fixture.runtime.runtime_epoch,
        sequence,
    );
    let before: ExitRecord =
        read_json(&fixture.host.paths.exit_record(fixture.manifest.terminal_id)).unwrap();

    fixture
        .host
        .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
        .await
        .unwrap();

    let after: ExitRecord =
        read_json(&fixture.host.paths.exit_record(fixture.manifest.terminal_id)).unwrap();
    assert_eq!(after, before);
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_rejects_non_crash_evidence() {
    let fixture = AutomaticCrashFixture::new();
    let sequence = fixture.append_output();
    fixture.settle_custody();
    fixture.write_exit(ExitReason::Natural, fixture.runtime.runtime_epoch, sequence);

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

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_fails_closed_for_custody_journal_and_socket() {
    let no_custody = AutomaticCrashFixture::new();
    assert!(
        no_custody
            .host
            .ensure_automatic_worker_crash_exit_inner(&no_custody.manifest)
            .await
            .is_err()
    );

    let no_journal = AutomaticCrashFixture::new();
    no_journal.settle_custody();
    no_journal.remove_journal_evidence();
    assert!(
        no_journal
            .host
            .ensure_automatic_worker_crash_exit_inner(&no_journal.manifest)
            .await
            .is_err()
    );

    let active_socket = AutomaticCrashFixture::new();
    active_socket.append_output();
    active_socket.settle_custody();
    let listener = tokio::net::UnixListener::bind(&active_socket.runtime.socket_path).unwrap();
    assert!(
        active_socket
            .host
            .ensure_automatic_worker_crash_exit_inner(&active_socket.manifest)
            .await
            .is_err()
    );
    drop(listener);

    let canonical_socket = AutomaticCrashFixture::new();
    canonical_socket.append_output();
    canonical_socket.settle_custody();
    let canonical_path = canonical_socket
        .host
        .paths
        .worker_socket(canonical_socket.manifest.terminal_id)
        .unwrap();
    let listener = tokio::net::UnixListener::bind(&canonical_path).unwrap();
    assert!(
        canonical_socket
            .host
            .ensure_automatic_worker_crash_exit_inner(&canonical_socket.manifest)
            .await
            .is_err()
    );
    drop(listener);
    std::fs::remove_file(canonical_path).unwrap();
}

#[cfg(target_os = "macos")]
#[tokio::test]
async fn automatic_worker_crash_exit_skips_previous_boot_runtime() {
    let fixture = AutomaticCrashFixture::new();
    let mut previous_boot = fixture.runtime.clone();
    previous_boot.boot_id = AUTOMATIC_EXIT_TEST_PREVIOUS_BOOT.into();
    atomic_write_json(
        &fixture
            .host
            .paths
            .runtime_record(fixture.manifest.terminal_id),
        &previous_boot,
    )
    .unwrap();

    fixture
        .host
        .ensure_automatic_worker_crash_exit_inner(&fixture.manifest)
        .await
        .unwrap();
    assert!(
        !fixture
            .host
            .paths
            .exit_record(fixture.manifest.terminal_id)
            .exists()
    );
}
