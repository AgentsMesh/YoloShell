use super::support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, deadline};
use super::tui_queries::{constants::*, driver::Driver};
use super::tui_workspaces::support as workspace;
use crate::Harness;
use yolo_hub_rpc_protocol::{Command, Reply};
#[path = "tui_creation_constants.rs"]
mod constants;
#[path = "tui_creation_support.rs"]
mod support;
use constants::*;
use workspace::DocumentPhase;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_creates_a_shell_in_its_directory_and_restores_read_only_binding() {
    let host = Harness::start().await;
    let baseline = host.create_cat(FIRST_NAME).await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let directory = root_path.join(CREATED_DIRECTORY);
    std::fs::create_dir(&directory).unwrap();
    let hub = RunningHub::start(&host, &root_path.join(STATE_DIRECTORY)).await;
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    let initial = workspace::document_at(
        &driver,
        &hub,
        DocumentPhase::InitialUnboundTab,
        |_| true,
    )
    .await;
    let original = initial.tabs.active();
    let original_id = original.id;
    let pane = original.view.layout.focused();
    assert!(
        original
            .view            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
    driver.send(NEW_SHELL).await;
    driver.wait(|text| text.contains(DIRECTORY_PROMPT)).await;
    driver.send(CLEAR_DEFAULT_DIRECTORY).await;
    driver.send(directory.to_str().unwrap().as_bytes()).await;
    driver.send(FINISH_SEARCH).await;
    let created = workspace::document_at(&driver, &hub, DocumentPhase::NewShellBinding, |saved| {
        let active = saved.tabs.active();
        active.id == original_id
            && active
                .view                .layout
                .panes()
                .iter()
                .any(|entry| entry.id == pane && entry.terminal.is_some())
    })
    .await;
    let address = support::binding(&created);
    assert_eq!(address.hub_id, hub.hub);
    assert_ne!(address.terminal_id.as_uuid(), baseline.manifest.terminal_id);
    let before = host
        .client()
        .get_session(address.terminal_id.as_uuid())
        .await
        .unwrap();
    host.track_view(&before);
    assert_eq!(before.manifest.cwd, directory);
    let Reply::Terminal { terminal: session } = hub
        .client()
        .await
        .call(hub.hub, Command::Get { address }, deadline())
        .await
        .unwrap()
    else {
        panic!()
    };
    let target = session.fields().runtime.unwrap().target;
    let name = &session.fields().name;
    // A live creation acquires a fresh writer without an extra open action.
    support::terminal_ready(&mut driver, name).await;
    driver.send(SHELL_OUTPUT).await;
    let output = format!("{SHELL_MARKER}:{}", directory.display());
    // The command does not contain this concatenated marker, so echo cannot pass.
    driver.wait(|text| text.contains(&output)).await;
    assert_eq!(
        support::binding(
            &workspace::document_at(
                &driver,
                &hub,
                DocumentPhase::ExistingBindingAfterOutput,
                |_| true
            )
            .await
        ),
        address
    );
    let lease = workspace::lease(&hub, target).await.unwrap();
    let mut observer = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    observer.open(name, INPUT_HELD).await;
    workspace::runtime_unchanged(&host, target, &before).await;
    let hidden_lease = workspace::lease(&hub, target).await.unwrap();
    assert_eq!(lease.fields().generation, hidden_lease.fields().generation);
    assert_eq!(lease.fields().holder_id, hidden_lease.fields().holder_id);
    observer.finish().await;
    let returned = workspace::document_at(
        &driver,
        &hub,
        DocumentPhase::ExistingBindingAfterOutput,
        |saved| saved.tabs.active().id == original_id,
    )
    .await;
    assert_eq!(support::binding(&returned), address);
    support::terminal_ready(&mut driver, name).await;
    driver.send(RETURN_OUTPUT).await;
    driver.wait(|text| text.contains(RETURN_MARKER)).await;
    workspace::runtime_unchanged(&host, target, &before).await;
    let stored = driver.finish().await;
    workspace::no_writer(&hub, target).await;
    let saved: yolo_tui_tab::TabDocument =
        yolo_runtime::paths::read_json(&stored.path().join(yolo_tui_tab::TAB_FILE))
            .unwrap();
    let mut restored = Driver::start(&driver_host, &hub, stored).await;
    // Persisted layout restores observation, not another process's write intent.
    restored.access(name, READ_ONLY).await;
    restored.wait(|text| text.contains(RETURN_MARKER)).await;
    let restored_document =
        workspace::document_at(&restored, &hub, DocumentPhase::RestoredTab, |_| true).await;
    workspace::assert_retained_layouts(&saved.tabs, &restored_document.tabs);
    assert_eq!(support::binding(&restored_document), address);
    workspace::no_writer(&hub, target).await;
    workspace::runtime_unchanged(&host, target, &before).await;
    support::exact_sessions(&host, baseline.manifest.terminal_id, address).await;
    restored.finish().await;
    hub.stop().await;
}
