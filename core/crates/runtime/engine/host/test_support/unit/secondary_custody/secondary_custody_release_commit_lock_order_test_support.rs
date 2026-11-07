use super::super::*;
use crate::pty_custody::{CustodyDurableRecord, CustodyRuntimeIdentity};
use std::sync::{Arc, mpsc};
use std::time::Duration;

const LOCK_ORDER_TEST_TIMEOUT: Duration = Duration::from_secs(2);
const CONTENTION_NOT_OBSERVED_DETAIL: &str =
    "durable retirement did not wait for the held lifecycle lock";
const IN_MEMORY_LOCKS_BLOCKED_DETAIL: &str =
    "durable retirement held in-memory custody locks while waiting for lifecycle";

pub(super) async fn assert_waiting_retirement_lock_order() {
    let (mut fixture, lease, server, finish) = super::ready_fixture().await;
    let runtime = CustodyRuntimeIdentity::from_record(&fixture.record);
    drop(super::settled(&mut fixture, &lease).await);
    lease.set_state(SecondaryCustodyState::Disconnected);

    let observation = observe_while_lifecycle_is_held(&fixture.host, &lease, runtime.clone());
    assert_eq!(
        observation,
        (true, true, false, SecondaryCustodyState::Disconnected, true,),
        "{IN_MEMORY_LOCKS_BLOCKED_DETAIL}"
    );

    let receipt: CustodyDurableRecord = fixture
        .host
        .namespace
        .read_json(&fixture.host.paths.pty_custody(runtime.terminal_id))
        .unwrap();
    assert!(receipt.is_valid());
    assert!(receipt.retirement_evidence().is_some());
    assert!(fixture.host.secondary_custody.current(&runtime).is_none());
    assert!(
        fixture
            .host
            .secondary_custody
            .retired_for_settlement(&runtime)
    );
    assert!(matches!(
        fixture.host.secondary_custody.admission(&runtime),
        SecondaryCustodyAdmission::Retired {
            state: SecondaryCustodyState::Released,
            ..
        }
    ));
    assert_eq!(lease.state(), SecondaryCustodyState::Released);
    assert!(!lease.retains_platform());
    assert!(fixture.host.request_drain.fail_stop_detail().is_none());
    finish.send(()).unwrap();
    server.await.unwrap();
}

fn observe_while_lifecycle_is_held(
    host: &Host,
    lease: &Arc<SecondaryCustodyLease>,
    runtime: CustodyRuntimeIdentity,
) -> (bool, bool, bool, SecondaryCustodyState, bool) {
    let lifecycle = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
        &host.paths,
        &host.namespace,
        runtime.terminal_id,
    )
    .unwrap();
    std::thread::scope(|scope| {
        let commit_lease = Arc::clone(lease);
        let commit = scope.spawn(move || {
            let settlement = settlement::prove_exact(&commit_lease).unwrap().unwrap();
            commit_lease.commit_settled_release(settlement)
        });
        let deadline = std::time::Instant::now() + LOCK_ORDER_TEST_TIMEOUT;
        while crate::paths::test_support::queued_session_lifecycle_waiters(
            &host.paths,
            runtime.terminal_id,
        )
        .unwrap()
            == 0
        {
            assert!(
                std::time::Instant::now() < deadline,
                "{CONTENTION_NOT_OBSERVED_DETAIL}"
            );
            std::thread::sleep(crate::host::PROCESS_EXIT_POLL_INTERVAL);
        }

        let (probe_tx, probe_rx) = mpsc::channel();
        let probe_lease = Arc::clone(lease);
        let probe = scope.spawn(move || {
            let current = host
                .secondary_custody
                .current(&runtime)
                .is_some_and(|current| Arc::ptr_eq(&current, &probe_lease));
            let admission = match host.secondary_custody.admission(&runtime) {
                SecondaryCustodyAdmission::Active {
                    lease: observed,
                    state,
                    ready,
                } => {
                    Arc::ptr_eq(&observed, &probe_lease)
                        && state == SecondaryCustodyState::Disconnected
                        && !ready
                }
                _ => false,
            };
            let retired = host.secondary_custody.retired_for_settlement(&runtime);
            let state = probe_lease.state();
            let platform = probe_lease.retains_platform();
            probe_tx
                .send((current, admission, retired, state, platform))
                .unwrap();
        });
        let observation = probe_rx.recv_timeout(LOCK_ORDER_TEST_TIMEOUT);
        drop(lifecycle);
        commit.join().unwrap().unwrap();
        probe.join().unwrap();
        observation.expect(IN_MEMORY_LOCKS_BLOCKED_DETAIL)
    })
}
