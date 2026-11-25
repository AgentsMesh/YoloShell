use super::support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, acquire, deadline};
use crate::Harness;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{LeaseGrant, LeaseMode};
#[path = "tui_queries_constants.rs"]
pub(super) mod constants;
#[path = "tui_queries_driver.rs"]
pub(super) mod driver;
#[path = "tui_queries_panes.rs"]
mod panes;
#[path = "tui_queries_source.rs"]
pub(super) mod source;
#[path = "tui_queries_takeover.rs"]
mod takeover;
use constants::*;
use driver::Driver;
use source::Source;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_pty_queries_require_current_writer_and_never_replay_across_takeover_or_restore() {
    let host = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let hub = RunningHub::start(
        &host,
        &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
    )
    .await;
    let mut source = Source::start(&host, hub.hub, FIRST_NAME).await;
    // Queries precede the first attach; restoring historical output must be quiet.
    assert!(source.emit(SNAPSHOT_QUERY).await.is_empty());
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    let lease = lease(&hub, source.target, LeaseMode::Acquire {}).await;
    driver.open(source.name, INPUT_HELD).await;
    assert!(source.emit(STANDARD_QUERIES).await.is_empty());
    assert!(source.emit(QUERY_PREFIX).await.is_empty());
    driver.send(TAKE_CONTROL).await;
    driver.access(source.name, CONTROL).await;
    assert!(source.emit(QUERY_SUFFIX).await.is_empty());
    let mut expected = [
        STATUS_REPLY,
        MIDDLE_CURSOR_REPLY,
        DEVICE_ATTRIBUTES_REPLY,
        DEVICE_ATTRIBUTES_REPLY,
    ]
    .concat();
    source.expect(STANDARD_QUERIES, &expected).await;
    assert_eq!(source.emit(UNSUPPORTED_QUERIES).await, expected);
    // A fragment within one valid writer scope is answered exactly once.
    assert_eq!(source.emit(QUERY_PREFIX).await, expected);
    expected.extend_from_slice(STATUS_REPLY);
    source.expect(STATUS_SUFFIX, &expected).await;
    assert_eq!(source.emit(QUERY_PREFIX).await, expected);
    eprintln!("terminal query phase: revoke writer with a fragmented query pending");
    let takeover = take_over(&hub, source.target).await;
    driver.send(CONTROL_CANARY).await;
    takeover::wait_for_revoked_writer(&mut driver, source.name, takeover).await;
    takeover::assert_external_writer(&hub, takeover).await;
    assert_eq!(source.emit(QUERY_SUFFIX).await, expected);
    assert_eq!(source.emit(STANDARD_QUERIES).await, expected);
    takeover::assert_external_writer(&hub, takeover).await;
    eprintln!("terminal query phase: explicit takeover grants a new query scope");
    driver.send(TAKE_CONTROL).await;
    driver.access(source.name, CONTROL).await;
    assert_eq!(source.emit(QUERY_SUFFIX).await, expected);
    expected.extend_from_slice(FIRST_CURSOR_REPLY);
    source.expect(FIRST_CURSOR_QUERY, &expected).await;
    let workspace = driver.finish().await;
    assert_eq!(source.emit(SNAPSHOT_QUERY).await, expected);
    eprintln!("terminal query phase: restore historical output without writer authority");
    let mut restored = Driver::start(&driver_host, &hub, workspace).await;
    // A new process restores observation only; live reconnect retains write intent.
    restored.access(source.name, READ_ONLY).await;
    assert_eq!(source.emit(EMPTY_OUTPUT).await, expected);
    let reply = hub
        .client()
        .await
        .call(
            hub.hub,
            Command::ObserveLease {
                target: source.target,
            },
            deadline(),
        )
        .await
        .unwrap();
    assert!(matches!(reply, Reply::LeaseObservation { lease: None }));
    restored.finish().await;
    // Stale cleanup can release only its own generation, never restored state.
    for grant in [lease, takeover] {
        let _ = hub
            .client()
            .await
            .call(hub.hub, Command::Release { grant }, deadline())
            .await;
    }
    hub.stop().await;
}

async fn lease(
    hub: &RunningHub,
    target: yolo_hub_identity::RuntimeTarget,
    mode: LeaseMode,
) -> LeaseGrant {
    let Reply::Lease { grant } = hub
        .client()
        .await
        .call(hub.hub, acquire(target, mode), deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    grant
}

async fn take_over(hub: &RunningHub, target: yolo_hub_identity::RuntimeTarget) -> LeaseGrant {
    let Reply::LeaseObservation {
        lease: Some(observed),
    } = hub
        .client()
        .await
        .call(hub.hub, Command::ObserveLease { target }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    lease(
        hub,
        target,
        LeaseMode::TakeControl {
            observed_generation: observed.fields().generation,
        },
    )
    .await
}
