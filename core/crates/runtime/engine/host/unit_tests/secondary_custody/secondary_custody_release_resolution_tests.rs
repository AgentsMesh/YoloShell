use super::*;

const INVALID_OUTCOME_SOCKET: &str = "invalid-release-outcome.sock";
const MISSING_REGISTRY_SOCKET: &str = "missing-release-registry.sock";
const MISSING_ENTRY_SOCKET: &str = "missing-release-entry.sock";
const REPLACED_ENTRY_SOCKET: &str = "replaced-release-entry.sock";
const REPLACEMENT_SOCKET: &str = "replacement-release-entry.sock";
const INVALID_STATE_SOCKET: &str = "invalid-release-state.sock";

#[tokio::test]
async fn pending_release_rejects_nonterminal_outcome() {
    let fixture = PendingReleaseFixture::new(INVALID_OUTCOME_SOCKET);

    assert_protocol(
        fixture
            .lease
            .resolve_pending_release(SecondaryCustodyState::Ready),
    );
    assert_eq!(
        fixture.lease.state(),
        SecondaryCustodyState::ReleaseAckPending
    );
}

#[tokio::test]
async fn pending_release_requires_live_registry() {
    let fixture = PendingReleaseFixture::new(MISSING_REGISTRY_SOCKET);
    *crate::host::lock_recover(&fixture.lease.registry) = None;

    assert_protocol(
        fixture
            .lease
            .resolve_pending_release(SecondaryCustodyState::Released),
    );
    assert_eq!(
        fixture.lease.state(),
        SecondaryCustodyState::ReleaseAckPending
    );
}

#[tokio::test]
async fn pending_release_requires_retirement_entry() {
    let fixture = PendingReleaseFixture::new(MISSING_ENTRY_SOCKET);
    let key = SecondaryCustodyKey::from_runtime(&fixture.lease.runtime);
    crate::host::lock_recover(&fixture.host.secondary_custody.entries.values)
        .retired
        .remove(&key);

    assert_protocol(
        fixture
            .lease
            .resolve_pending_release(SecondaryCustodyState::Released),
    );
}

#[tokio::test]
async fn pending_release_rejects_replaced_retirement_identity() {
    let fixture = PendingReleaseFixture::new(REPLACED_ENTRY_SOCKET);
    let replacement = fixture.pending_lease(REPLACEMENT_SOCKET);
    let key = SecondaryCustodyKey::from_runtime(&fixture.lease.runtime);
    let replacement_key = SecondaryCustodyKey::from_runtime(&replacement.runtime);
    let mut values = crate::host::lock_recover(&fixture.host.secondary_custody.entries.values);
    let replacement = values.retired.remove(&replacement_key).unwrap();
    values.retired.insert(key, replacement);
    drop(values);

    assert_protocol(
        fixture
            .lease
            .resolve_pending_release(SecondaryCustodyState::Released),
    );
}

#[tokio::test]
async fn pending_release_requires_ack_pending_state() {
    let fixture = PendingReleaseFixture::new(INVALID_STATE_SOCKET);
    *crate::host::lock_recover(&fixture.lease.state) = SecondaryCustodyState::Ready;

    assert_protocol(
        fixture
            .lease
            .resolve_pending_release(SecondaryCustodyState::Released),
    );
    assert_eq!(fixture.lease.state(), SecondaryCustodyState::Ready);
}

struct PendingReleaseFixture {
    host: Host,
    lease: Arc<SecondaryCustodyLease>,
    directory: HostTestDirectory,
}

impl PendingReleaseFixture {
    fn new(socket_name: &str) -> Self {
        let directory = tempdir().unwrap();
        let host = Host::open(crate::HostConfig::new(directory.path())).unwrap();
        let lease = pending_lease(&host, directory.path(), socket_name);
        Self {
            directory: HostTestDirectory::capture(directory, &host.paths),
            host,
            lease,
        }
    }

    fn pending_lease(&self, socket_name: &str) -> Arc<SecondaryCustodyLease> {
        pending_lease(&self.host, self.directory.path(), socket_name)
    }
}

fn pending_lease(
    host: &Host,
    directory: &std::path::Path,
    socket_name: &str,
) -> Arc<SecondaryCustodyLease> {
    let record = runtime(host, directory.join(socket_name));
    host.secondary_custody.install_test_retired_lease(
        host,
        &record,
        TestRetiredCustodyState::ReleaseAckPending,
    )
}

fn assert_protocol(result: Result<(), WorkerRpcError>) {
    assert!(matches!(result, Err(WorkerRpcError::Protocol(_))));
}
