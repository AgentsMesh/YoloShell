use crate::host::tests::*;
const RPC_TEST_PID_OFFSET: u32 = 1;
const RPC_TEST_BOOT_SUFFIX: char = 'x';

#[test]
fn rpc_test_custody_authority_is_exact_and_host_local() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), host.socket_path().to_owned());
    assert!(host.secondary_custody.rpc_test_authorized(&record));
    assert!(host.secondary_custody.rpc_test_authorized(&record.clone()));

    let mut replaced = record.clone();
    replaced.runtime_epoch = Uuid::new_v4();
    assert!(!host.secondary_custody.rpc_test_authorized(&replaced));
    replaced = record.clone();
    replaced.worker_pid = replaced.worker_pid.saturating_add(RPC_TEST_PID_OFFSET);
    assert!(!host.secondary_custody.rpc_test_authorized(&replaced));
    replaced = record.clone();
    replaced.boot_id.push(RPC_TEST_BOOT_SUFFIX);
    assert!(!host.secondary_custody.rpc_test_authorized(&replaced));

    let other_directory = tempdir().unwrap();
    let other = Host::open(HostConfig::new(other_directory.path())).unwrap();
    assert!(!other.secondary_custody.rpc_test_authorized(&record));
}

#[test]
fn fixture_peer_authority_is_exact_and_host_local() {
    let directory = tempdir().unwrap();
    let host = Host::open(HostConfig::new(directory.path())).unwrap();
    let record = host_runtime_record(&host, Uuid::new_v4(), host.socket_path().to_owned());
    let peer = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    host.secondary_custody
        .authorize_peer_for_test(&record, peer);
    assert_eq!(
        host.secondary_custody.authorized_peer_for_test(&record),
        Some(peer)
    );

    let mut replaced = record.clone();
    replaced.worker_pid = replaced.worker_pid.saturating_add(RPC_TEST_PID_OFFSET);
    assert_eq!(
        host.secondary_custody.authorized_peer_for_test(&replaced),
        None
    );

    let other_directory = tempdir().unwrap();
    let other = Host::open(HostConfig::new(other_directory.path())).unwrap();
    assert_eq!(
        other.secondary_custody.authorized_peer_for_test(&record),
        None
    );
}
