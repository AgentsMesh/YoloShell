use crate::host::tests::*;
use super::*;

const RELEASED_EXIT_SOCKET_NAME: &str = "released-exit.sock";
pub(super) const EXIT_CODE: i32 = 0;
pub(super) const EXIT_OBSERVED_AT_MS: i64 = 1;
pub(super) const EXIT_OUTPUT_SEQUENCE: u64 = 1;
const STALE_EPOCH_ERROR_DETAIL: &str = "stale released exit must fail closed";
const RELEASED_EXIT_REPLACEMENT_AUTH_TOKEN: &str = "replacement-auth-token";
const RELEASED_EXIT_REPLACEMENT_SOCKET_NAME: &str = "replacement-runtime.sock";
const CONTROLLER_GENERATION_INCREMENT: u64 = 1;

pub(super) fn released_exit_fixture(reason: ExitReason) -> (tempfile::TempDir, Host, RuntimeRecord) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut record = host_runtime_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(RELEASED_EXIT_SOCKET_NAME),
    );
    host.paths.ensure_session(record.terminal_id).unwrap();
    let identity = reaped_released_exit_child_identity();
    record.child_pid = identity.pid;
    record.child_start_time = identity.start_time;
    atomic_write_json(&host.paths.runtime_record(record.terminal_id), &record).unwrap();
    let exit = ExitRecord {
        runtime_epoch: record.runtime_epoch,
        code: Some(EXIT_CODE),
        signal: None,
        reason,
        observed_at_ms: EXIT_OBSERVED_AT_MS,
        last_output_sequence: EXIT_OUTPUT_SEQUENCE,
    };
    atomic_write_json(&host.paths.exit_record(record.terminal_id), &exit).unwrap();
    let _lease = host.secondary_custody.install_test_retired_lease(
        &host,
        &record,
        crate::host::secondary_custody::TestRetiredCustodyState::Released,
    );
    (directory, host, record)
}

#[tokio::test]
async fn exact_natural_exit_authorizes_only_the_released_read_only_path() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let expected: ExitRecord = read_json(&host.paths.exit_record(record.terminal_id)).unwrap();

    assert!(host.released_natural_attachment_evidence(&record));
    let authenticated = host
        .validate_released_natural_attachment(&record)
        .await
        .unwrap();
    assert_eq!(authenticated, expected);
    host.validate_released_terminal_worker_shutdown(&record)
        .unwrap();
    assert!(host.custodied_worker_stream(&record).await.is_err());
}

#[tokio::test]
async fn non_natural_exit_never_authorizes_read_only_attachment() {
    for reason in [
        ExitReason::UserTerminate,
        ExitReason::WorkerCrash,
        ExitReason::HostReboot,
        ExitReason::Lost,
    ] {
        let (_directory, host, record) = released_exit_fixture(reason);
        assert!(!host.released_natural_attachment_evidence(&record));
        assert!(host.released_terminal_worker_shutdown_evidence(&record));
        let error = host
            .validate_released_natural_attachment(&record)
            .await
            .unwrap_err();
        assert_eq!(error.code, crate::protocol::error_code::IDENTITY_MISMATCH);
        assert!(!error.retryable);
    }
}

#[tokio::test]
async fn stale_exit_live_child_and_replaced_runtime_fail_closed() {
    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let mut stale = read_json::<ExitRecord>(&host.paths.exit_record(record.terminal_id)).unwrap();
    stale.runtime_epoch = Uuid::new_v4();
    atomic_write_json(&host.paths.exit_record(record.terminal_id), &stale).unwrap();
    assert!(
        !host.released_natural_attachment_evidence(&record),
        "{STALE_EPOCH_ERROR_DETAIL}"
    );

    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let mut live_child = record.clone();
    live_child.child_pid = std::process::id();
    live_child.child_start_time = process::process_start_time(std::process::id()).unwrap();
    atomic_write_json(&host.paths.runtime_record(record.terminal_id), &live_child).unwrap();
    assert!(!host.released_natural_attachment_evidence(&live_child));

    let (_directory, host, record) = released_exit_fixture(ExitReason::Natural);
    let mut replacement = record.clone();
    replacement.worker_id = Uuid::new_v4();
    atomic_write_json(&host.paths.runtime_record(record.terminal_id), &replacement).unwrap();
    assert!(!host.released_natural_attachment_evidence(&record));

    for replacement in [
        {
            let mut replacement = record.clone();
            replacement.auth_token = RELEASED_EXIT_REPLACEMENT_AUTH_TOKEN.into();
            replacement
        },
        {
            let mut replacement = record.clone();
            replacement.socket_path = host
                .paths
                .home()
                .join(RELEASED_EXIT_REPLACEMENT_SOCKET_NAME);
            replacement
        },
        {
            let mut replacement = record.clone();
            replacement.controller_generation = replacement
                .controller_generation
                .checked_add(CONTROLLER_GENERATION_INCREMENT)
                .unwrap();
            replacement
        },
    ] {
        atomic_write_json(&host.paths.runtime_record(record.terminal_id), &replacement).unwrap();
        assert!(!host.released_natural_attachment_evidence(&record));
    }
}

pub(super) fn reaped_released_exit_child_identity() -> crate::process::ProcessIdentity {
    crate::host::reaped_host_test_process_identity()
}
