use super::*;
use crate::pty_custody::CustodyRuntimeIdentity;

const LATE_ACK_PREDECESSOR_SOCKET: &str = "late-ack-predecessor.sock";
const LATE_ACK_SUCCESSOR_SOCKET: &str = "late-ack-successor.sock";
const LATE_SUPERSEDE_PREDECESSOR_SOCKET: &str = "late-supersede-predecessor.sock";
const LATE_SUPERSEDE_SUCCESSOR_SOCKET: &str = "late-supersede-successor.sock";
const EXPECTED_RETAINED_RETIREMENTS: usize = 0;
const EXPECTED_TERMINAL_RETIREMENTS: usize = 1;

#[tokio::test]
async fn late_predecessor_ack_preserves_newer_terminal_tombstone() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let predecessor_record = generation(
        &host,
        directory.path(),
        terminal_id,
        LATE_ACK_PREDECESSOR_SOCKET,
    );
    let predecessor_identity = CustodyRuntimeIdentity::from_record(&predecessor_record);
    let predecessor = host.secondary_custody.install_test_retired_lease(
        &host,
        &predecessor_record,
        TestRetiredCustodyState::ReleaseAckPending,
    );
    let successor_record = generation(
        &host,
        directory.path(),
        terminal_id,
        LATE_ACK_SUCCESSOR_SOCKET,
    );
    let successor_identity = CustodyRuntimeIdentity::from_record(&successor_record);
    let successor = host.secondary_custody.install_test_retired_lease(
        &host,
        &successor_record,
        TestRetiredCustodyState::Superseded,
    );

    pending_release::PendingReleaseAck::new(&predecessor).acknowledge();

    assert_absent(&host, &predecessor_identity);
    assert_terminal(&host, &successor_identity);
    assert_compacted(&host);
    assert_lease_unretained(predecessor);
    assert_lease_unretained(successor);
}

#[tokio::test]
async fn late_predecessor_supersede_preserves_newer_terminal_tombstone() {
    let directory = tempdir().unwrap();
    let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
    let terminal_id = Uuid::new_v4();
    let predecessor_record = generation(
        &host,
        directory.path(),
        terminal_id,
        LATE_SUPERSEDE_PREDECESSOR_SOCKET,
    );
    let predecessor_identity = CustodyRuntimeIdentity::from_record(&predecessor_record);
    let predecessor = host
        .secondary_custody
        .install_test_active_lease(&host, &predecessor_record);
    let successor_record = generation(
        &host,
        directory.path(),
        terminal_id,
        LATE_SUPERSEDE_SUCCESSOR_SOCKET,
    );
    let successor_identity = CustodyRuntimeIdentity::from_record(&successor_record);
    let successor = host.secondary_custody.install_test_retired_lease(
        &host,
        &successor_record,
        TestRetiredCustodyState::Superseded,
    );

    predecessor.commit_superseded().unwrap();

    assert_absent(&host, &predecessor_identity);
    assert_terminal(&host, &successor_identity);
    assert_compacted(&host);
    assert_lease_unretained(predecessor);
    assert_lease_unretained(successor);
}

fn generation(
    host: &Host,
    directory: &std::path::Path,
    terminal_id: Uuid,
    socket_name: &str,
) -> RuntimeRecord {
    let mut record = runtime(host, directory.join(socket_name));
    record.terminal_id = terminal_id;
    record
}

fn assert_absent(host: &Host, runtime: &CustodyRuntimeIdentity) {
    assert!(matches!(
        host.secondary_custody.admission(runtime),
        SecondaryCustodyAdmission::Absent
    ));
    assert!(!host.secondary_custody.retired_for_settlement(runtime));
}

fn assert_terminal(host: &Host, runtime: &CustodyRuntimeIdentity) {
    assert!(matches!(
        host.secondary_custody.admission(runtime),
        SecondaryCustodyAdmission::Retired {
            state: SecondaryCustodyState::Superseded,
            ..
        }
    ));
    assert!(host.secondary_custody.retired_for_settlement(runtime));
}

fn assert_compacted(host: &Host) {
    assert_eq!(
        host.secondary_custody.retained_retirement_count(),
        EXPECTED_RETAINED_RETIREMENTS
    );
    assert_eq!(
        host.secondary_custody.terminal_retirement_count(),
        EXPECTED_TERMINAL_RETIREMENTS
    );
}

fn assert_lease_unretained(lease: Arc<SecondaryCustodyLease>) {
    let retired = Arc::downgrade(&lease);
    drop(lease);
    assert!(retired.upgrade().is_none());
}
