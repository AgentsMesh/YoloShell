use super::support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, deadline};
use super::tui_queries::{constants::*, driver::Driver, source::Source};
use crate::Harness;
#[path = "tui_workspaces_constants.rs"]
pub(super) mod constants;
#[path = "tui_workspaces_layout.rs"]
mod layout;
#[path = "tui_workspaces_support.rs"]
pub(super) mod support;
use constants::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_independent_clients_mouse_zoom_and_detach_preserve_shell_custody() {
    let host = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let hub = RunningHub::start(
        &host,
        &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
    )
    .await;
    let mut first = Source::start(&host, hub.hub, FIRST_NAME).await;
    let mut second = Source::start(&host, hub.hub, SECOND_NAME).await;
    let first_before = host
        .client()
        .get_session(first.target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let second_before = host
        .client()
        .get_session(second.target.address.terminal_id.as_uuid())
        .await
        .unwrap();
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    layout::exercise(&mut driver, &hub, &host, &mut first, &second).await;
    first.expect(FIRST_CURSOR_QUERY, FIRST_CURSOR_REPLY).await;
    let first_lease = support::lease(&hub, first.target).await.unwrap();
    let original_layout = support::document(&driver, &hub, |_| true).await.tabs;
    let mut other = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    let hidden_expected = [FIRST_CURSOR_REPLY, FIRST_CURSOR_REPLY].concat();
    first.expect(FIRST_CURSOR_QUERY, &hidden_expected).await;
    assert_eq!(first.emit(HIDDEN_OUTPUT).await, hidden_expected);
    let hidden_lease = support::lease(&hub, first.target).await.unwrap();
    assert_eq!(
        first_lease.fields().generation,
        hidden_lease.fields().generation
    );
    assert_eq!(
        first_lease.fields().holder_id,
        hidden_lease.fields().holder_id
    );
    if let Err(error) = other.try_open(second.name, CONTROL).await {
        let (first_source, second_source, frontend) =
            tokio::join!(first.diagnostic(), second.diagnostic(), other.diagnostic(),);
        panic!(
            "second client binding failed: {error}; first={first_source}; second={second_source}; frontend={frontend}; host={:?} alive={} stderr={}",
            host.host_identity(),
            host.host_identity().matches(),
            host.host_stderr_tail()
        );
    }
    other
        .focused(hub.hub, second.target.address, second.name)
        .await;
    second
        .expect(SECOND_CURSOR_QUERY, SECOND_CURSOR_REPLY)
        .await;
    assert_eq!(
        support::document(&driver, &hub, |_| true).await.tabs,
        original_layout
    );
    driver
        .focused(hub.hub, first.target.address, first.name)
        .await;
    driver.wait(|text| text.contains(HIDDEN_MARKER)).await;
    // A second client may observe the first shell but cannot steal its writer.
    other.open(first.name, INPUT_HELD).await;
    let retained = support::lease(&hub, first.target).await.unwrap();
    assert_eq!(
        retained.fields().generation,
        first_lease.fields().generation
    );
    assert_eq!(retained.fields().holder_id, first_lease.fields().holder_id);
    other.finish().await;
    support::no_writer(&hub, second.target).await;
    support::runtime_unchanged(&host, first.target, &first_before).await;
    support::runtime_unchanged(&host, second.target, &second_before).await;
    assert_eq!(second.emit(SNAPSHOT_QUERY).await, SECOND_CURSOR_REPLY);
    let workspace = driver.finish().await;
    let saved: yolo_tui_tab::TabDocument =
        yolo_runtime::paths::read_json(&workspace.path().join(yolo_tui_tab::TAB_FILE))
            .unwrap();
    let mut restored = Driver::start(&driver_host, &hub, workspace).await;
    // Saved tabs do not persist input authority or live reconnection intent.
    restored.access(first.name, READ_ONLY).await;
    support::assert_retained_layouts(
        &saved.tabs,
        &support::document(&restored, &hub, |_| true)
            .await
            .tabs,
    );
    support::no_writer(&hub, first.target).await;
    assert_eq!(first.emit(SNAPSHOT_QUERY).await, hidden_expected);
    assert!(!restored.screen().contents().contains("TabCatalog"));
    restored.finish().await;
    support::runtime_unchanged(&host, first.target, &first_before).await;
    support::runtime_unchanged(&host, second.target, &second_before).await;
    hub.stop().await;
}
