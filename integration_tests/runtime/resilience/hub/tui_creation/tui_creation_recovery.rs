use super::{
    support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, deadline},
    tui_queries::{constants::FIRST_NAME, driver::Driver},
    tui_workspaces::support as workspace,
};
use crate::Harness;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget};
#[path = "tui_creation_recovery_audit.rs"]
mod audit;
#[path = "tui_creation_recovery/constants.rs"]
pub(super) mod constants;
#[path = "tui_creation_recovery_io.rs"]
mod io;
#[path = "tui_creation_recovery_journal.rs"]
mod journal;
#[path = "tui_creation_recovery_proxy.rs"]
mod proxy;
use constants::*;

#[tokio::test(flavor = "multi_thread")]
async fn lost_created_reply_survives_real_tui_crash_and_restarts_with_query_only() {
    let host = Harness::start().await;
    let baseline = host.create_cat(FIRST_NAME).await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let directory = root.path().canonicalize().unwrap().join(CREATED_DIRECTORY);
    std::fs::create_dir(&directory).unwrap();
    let mut hub = RunningHub::start(
        &host,
        &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
    )
    .await;
    let stored = tempfile::tempdir().unwrap();
    let proxy = proxy::Proxy::start(&hub.socket, stored.path());
    hub.socket = proxy.socket.clone();
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, stored).await;
    journal::unbound(&workspace::document(&driver, &hub, |_| true).await);
    driver.send(NEW_SHELL).await;
    driver.wait(|text| text.contains(DIRECTORY_PROMPT)).await;
    driver.send(CLEAR_DEFAULT_DIRECTORY).await;
    driver.send(directory.to_str().unwrap().as_bytes()).await;
    driver.send(SUBMIT_DIRECTORY).await;
    let (intent, creation) = proxy.lost_creation().await;
    assert_eq!(intent.origin, hub.hub);
    assert_eq!(intent.request.fields().cwd, directory.to_str().unwrap());
    let before = host
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    host.track_view(&before);
    assert_eq!(before.manifest.cwd, directory);
    let target = RuntimeTarget {
        address: creation.address,
        runtime_epoch: RuntimeEpoch::from_uuid(before.runtime.as_ref().unwrap().runtime_epoch)
            .unwrap(),
    };
    let stored = driver.crash_for_creation_recovery(&hub).await;
    proxy.old_connection_closed().await;
    let before_restart = journal::retained(stored.path(), hub.hub, &intent);
    workspace::no_writer(&hub, target).await;
    workspace::runtime_unchanged(&host, target, &before).await;
    let restored = Driver::start(&driver_host, &hub, stored).await;
    proxy.finalized().await;
    tokio::time::timeout_at(deadline(), async {
        while !journal::cleared(restored.workspace_path()) {
            tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
        }
    })
    .await
    .expect("recovered intent must become an acknowledged original finalization");
    let after_restart = workspace::document(&restored, &hub, |_| true).await;
    journal::unbound(&after_restart);
    workspace::assert_retained_layouts(&before_restart.tabs, &after_restart.tabs);
    workspace::no_writer(&hub, target).await;
    workspace::runtime_unchanged(&host, target, &before).await;
    let terminals = host.client().list_sessions_complete(false).await.unwrap();
    assert_eq!(terminals.len(), BASELINE_AND_CREATED);
    assert!(
        terminals
            .iter()
            .any(|view| view.manifest.terminal_id == baseline.manifest.terminal_id)
    );
    assert!(
        terminals
            .iter()
            .any(|view| view.manifest.terminal_id == creation.address.terminal_id.as_uuid())
    );
    let stored = restored.finish().await;
    journal::settled(stored.path(), hub.hub);
    workspace::no_writer(&hub, target).await;
    io::alive_with_fresh_output(&host, &before).await;
    workspace::no_writer(&hub, target).await;
    workspace::runtime_unchanged(&host, target, &before).await;
    proxy.stop().await;
    hub.stop().await;
}
