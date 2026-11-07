use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

const TERMINAL_RETIREMENT_CHURN_COUNT: usize = 128;
const TERMINAL_RETIREMENT_SOCKET_PREFIX: &str = "terminal-retirement";
const RELEASE_ACK_SOCKET_PREFIX: &str = "release-ack-retirement";
const RETAINED_RELEASE_SOCKET_NAME: &str = "retained-release-uncertain.sock";
const SUCCESSOR_SOCKET_NAME: &str = "retained-release-successor.sock";
const SOCKET_SUFFIX: &str = "sock";

#[tokio::test]
async fn terminal_churn_is_bounded_and_evicted_runtime_fails_durable_validation() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let mut oldest = None;
    let mut latest = None;

    for index in 0..TERMINAL_RETIREMENT_CHURN_COUNT {
        let record = generation(
            &host,
            directory.path(),
            terminal_id,
            TERMINAL_RETIREMENT_SOCKET_PREFIX,
            index,
        );
        let identity = CustodyRuntimeIdentity::from_record(&record);
        let lease = host.secondary_custody.install_test_retired_lease(
            &host,
            &record,
            TestRetiredCustodyState::Superseded,
        );
        let retired = Arc::downgrade(&lease);
        drop(lease);

        assert!(retired.upgrade().is_none());
        assert!(matches!(
            host.secondary_custody.admission(&identity),
            SecondaryCustodyAdmission::Retired {
                state: SecondaryCustodyState::Superseded,
                ..
            }
        ));
        assert_bounded(&host, 0);
        oldest.get_or_insert_with(|| record.clone());
        latest = Some(record);
    }
    let oldest = oldest.unwrap();
    let latest = latest.unwrap();
    let oldest_identity = CustodyRuntimeIdentity::from_record(&oldest);
    let latest_identity = CustodyRuntimeIdentity::from_record(&latest);
    assert!(matches!(
        host.secondary_custody.admission(&oldest_identity),
        SecondaryCustodyAdmission::Absent
    ));
    assert!(
        host.secondary_custody
            .retired_for_settlement(&latest_identity)
    );
    persist_pending(&host, &latest);

    let error = host.ensure_secondary_custody(&oldest).await.unwrap_err();
    assert!(matches!(error, WorkerRpcError::IdentityMismatch(_)));
    assert!(host.secondary_custody.current(&oldest_identity).is_none());
}

#[tokio::test]
async fn acknowledged_release_churn_compacts_terminal_tombstones() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();

    for index in 0..TERMINAL_RETIREMENT_CHURN_COUNT {
        let record = generation(
            &host,
            directory.path(),
            terminal_id,
            RELEASE_ACK_SOCKET_PREFIX,
            index,
        );
        let lease = host.secondary_custody.install_test_retired_lease(
            &host,
            &record,
            TestRetiredCustodyState::ReleaseAckPending,
        );
        let retired = Arc::downgrade(&lease);

        pending_release::PendingReleaseAck::new(&lease).acknowledge();
        assert_eq!(lease.state(), SecondaryCustodyState::Released);
        drop(lease);
        assert!(retired.upgrade().is_none());
        assert_bounded(&host, 0);
    }
}

#[tokio::test]
async fn terminal_churn_preserves_uncertain_release_predecessor() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let uncertain_record = named_generation(
        &host,
        directory.path(),
        terminal_id,
        RETAINED_RELEASE_SOCKET_NAME,
    );
    let uncertain = host.secondary_custody.install_test_retired_lease(
        &host,
        &uncertain_record,
        TestRetiredCustodyState::ReleaseUncertain,
    );
    let retained = Arc::downgrade(&uncertain);
    drop(uncertain);

    for index in 0..TERMINAL_RETIREMENT_CHURN_COUNT {
        let record = generation(
            &host,
            directory.path(),
            terminal_id,
            TERMINAL_RETIREMENT_SOCKET_PREFIX,
            index,
        );
        let lease = host.secondary_custody.install_test_retired_lease(
            &host,
            &record,
            TestRetiredCustodyState::Superseded,
        );
        drop(lease);
        assert_bounded(&host, 1);
    }
    let retained = retained.upgrade().unwrap();
    let successor = named_generation(&host, directory.path(), terminal_id, SUCCESSOR_SOCKET_NAME);
    let predecessors = host
        .secondary_custody
        .predecessors(&CustodyRuntimeIdentity::from_record(&successor));
    assert_eq!(predecessors.len(), 1);
    assert!(Arc::ptr_eq(&predecessors[0], &retained));
}

fn generation(
    host: &Host,
    directory: &std::path::Path,
    terminal_id: Uuid,
    prefix: &str,
    index: usize,
) -> RuntimeRecord {
    named_generation(
        host,
        directory,
        terminal_id,
        &format!("{prefix}-{index}.{SOCKET_SUFFIX}"),
    )
}

fn named_generation(
    host: &Host,
    directory: &std::path::Path,
    terminal_id: Uuid,
    socket_name: &str,
) -> RuntimeRecord {
    let mut record = runtime(host, directory.join(socket_name));
    record.terminal_id = terminal_id;
    record
}

fn assert_bounded(host: &Host, retained_count: usize) {
    assert_eq!(
        host.secondary_custody.retained_retirement_count(),
        retained_count
    );
    assert_eq!(host.secondary_custody.terminal_retirement_count(), 1);
}
