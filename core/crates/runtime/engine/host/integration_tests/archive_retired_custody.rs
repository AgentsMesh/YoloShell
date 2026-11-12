use crate::host::tests::*;
use crate::host::secondary_custody::TestRetiredCustodyState;

const RETIRED_ARCHIVE_SOCKET_NAME: &str = "archive-retired.sock";
const RETIRED_ARCHIVE_IDENTITY_OFFSET: u64 = 1;

#[tokio::test]
async fn archive_settles_released_and_superseded_custody_from_durable_evidence() {
    for state in retired_custody_states() {
        let (_directory, host, record) = retired_archive_fixture(state);
        assert!(!host.secondary_custody.rpc_test_authorized(&record));
        assert!(host.secondary_custody.retired_for_settlement(
            &crate::pty_custody::CustodyRuntimeIdentity::from_record(&record)
        ));
        host.settle_archive_runtime_inner(&record, ARCHIVE_TEST_TIMEOUT)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn retired_custody_archive_remains_fail_closed_for_each_durable_proof() {
    for state in retired_custody_states() {
        let (_directory, host, record) = retired_archive_fixture(state);
        let mut changed = record.clone();
        changed.worker_id = Uuid::new_v4();
        atomic_write_json(&host.paths.runtime_record(record.terminal_id), &changed).unwrap();
        assert_unsettled(&host, &record).await;
        atomic_write_json(&host.paths.runtime_record(record.terminal_id), &record).unwrap();

        let listener = UnixListener::bind(&record.socket_path).unwrap();
        assert_unsettled(&host, &record).await;
        drop(listener);

        persist_archive_exit(&host, &record, Uuid::new_v4());
        assert_unsettled(&host, &record).await;
    }
}

#[tokio::test]
async fn retired_custody_archive_still_requires_exact_process_exit() {
    let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    for state in retired_custody_states() {
        let (_directory, host, record) = retired_archive_fixture_with(state, |record| {
            record.worker_pid = process.pid;
            record.worker_start_time = process.start_time;
        });
        assert_unsettled(&host, &record).await;
    }

    let (_directory, host, record) =
        retired_archive_fixture_with(TestRetiredCustodyState::Superseded, |record| {
            record.child_pid = process.pid;
            record.child_start_time = process.start_time;
        });
    assert_unsettled(&host, &record).await;
}

#[tokio::test]
async fn retired_custody_archive_authority_requires_the_complete_runtime_identity() {
    let (_directory, host, record) = retired_archive_fixture(TestRetiredCustodyState::Superseded);
    let mut replaced = record.clone();
    replaced.child_start_time = replaced
        .child_start_time
        .saturating_add(RETIRED_ARCHIVE_IDENTITY_OFFSET);

    assert!(!host.secondary_custody.retired_for_settlement(
        &crate::pty_custody::CustodyRuntimeIdentity::from_record(&replaced)
    ));
}

fn retired_archive_fixture(
    state: TestRetiredCustodyState,
) -> (tempfile::TempDir, Host, RuntimeRecord) {
    retired_archive_fixture_with(state, |_| {})
}

fn retired_archive_fixture_with(
    state: TestRetiredCustodyState,
    prepare: impl FnOnce(&mut RuntimeRecord),
) -> (tempfile::TempDir, Host, RuntimeRecord) {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let mut record = dead_archive_record(
        &host,
        Uuid::new_v4(),
        directory.path().join(RETIRED_ARCHIVE_SOCKET_NAME),
    );
    host.secondary_custody.revoke_rpc_test(&record);
    prepare(&mut record);
    host.paths.ensure_session(record.terminal_id).unwrap();
    atomic_write_json(&host.paths.runtime_record(record.terminal_id), &record).unwrap();
    persist_archive_exit(&host, &record, record.runtime_epoch);
    host.secondary_custody
        .install_test_retired_lease(&host, &record, state);
    (directory, host, record)
}

fn retired_custody_states() -> [TestRetiredCustodyState; 3] {
    [
        TestRetiredCustodyState::Released,
        TestRetiredCustodyState::ReleaseUncertain,
        TestRetiredCustodyState::Superseded,
    ]
}

async fn assert_unsettled(host: &Host, record: &RuntimeRecord) {
    let error = host
        .settle_archive_runtime_inner(record, ARCHIVE_TEST_TIMEOUT)
        .await
        .unwrap_err();
    assert_eq!(error.message, ARCHIVE_RUNTIME_UNSETTLED);
}
