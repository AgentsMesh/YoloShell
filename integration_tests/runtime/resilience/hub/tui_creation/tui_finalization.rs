use super::support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY, deadline};
use super::tui_queries::{constants::FIRST_NAME, driver::Driver};
use super::tui_workspaces::support as workspace;
use crate::Harness;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreateShellFields, CreateShellRequest, TerminalSize};
use yolo_tui_creation_journal::{CreationJournal, PendingFinalization};
use yolo_tui_tab::TabStore;

#[path = "tui_finalization_support.rs"]
mod support;
use support::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_finishes_restored_creations_and_lost_acknowledgements_without_authority() {
    let host = Harness::start().await;
    let baseline = host.create_cat(FIRST_NAME).await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let root_path = root.path().canonicalize().unwrap();
    let hub = RunningHub::start(&host, &root_path.join(STATE_DIRECTORY)).await;
    let request = CreateShellRequest::new(CreateShellFields {
        schema_version: yolo_hub_terminal_protocol::TERMINAL_PROTOCOL_SCHEMA_VERSION,
        request_id: uuid::Uuid::new_v4(),
        creation_token: uuid::Uuid::new_v4(),
        name: Some(CREATED_NAME.into()),
        cwd: host.home.path().to_string_lossy().into_owned(),
        size: TerminalSize::default(),
    })
    .unwrap();
    let Reply::Created { creation } = hub
        .client()
        .await
        .call(
            hub.hub,
            Command::Create {
                request: request.clone(),
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("expected the original shell creation result")
    };
    let before = host
        .client()
        .get_session(creation.address.terminal_id.as_uuid())
        .await
        .unwrap();
    host.track_view(&before);
    assert_unfinalized(&host, creation.address).await;
    let Reply::Terminal { terminal: session } = hub
        .client()
        .await
        .call(
            hub.hub,
            Command::Get {
                address: creation.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("expected created shell")
    };
    let target = session.fields().runtime.unwrap().target;
    let record = PendingFinalization::new(hub.hub, request, creation).unwrap();
    let driver_host = Harness::start().await;
    let mut saved = tempfile::tempdir().unwrap();
    for _ in [Recovery::Unfinalized, Recovery::AcknowledgementLost] {
        // On the second pass the Host already has its durable Finalized receipt.
        // Keeping the identical record models a caller that did not retain its ACK.
        let store = TabStore::open(saved.path(), hub.hub).unwrap();
        let mut journal = CreationJournal::open(store.directory().unwrap(), hub.hub).unwrap();
        journal.retain_confirmed(record.clone()).unwrap();
        drop(journal);
        drop(store);
        let driver = Driver::start(&driver_host, &hub, saved).await;
        wait_acknowledged(driver.workspace_path()).await;
        // Prove the origin has finalized on each pass. An incorrectly cleared
        // journal must not let the second pass hide a missing first Finalize.
        assert_finalized_running(&host, creation.address).await;
        let document = workspace::document(&driver, &hub, |_| true).await;
        assert!(
            document
                .tabs
                .active()
                .view                .layout
                .panes()
                .iter()
                .all(|pane| pane.terminal.is_none())
        );
        workspace::no_writer(&hub, target).await;
        workspace::runtime_unchanged(&host, target, &before).await;
        assert_eq!(
            host.client()
                .list_sessions_complete(false)
                .await
                .unwrap()
                .len(),
            BASELINE_AND_CREATED
        );
        saved = driver.finish().await;
        let store = TabStore::open(saved.path(), hub.hub).unwrap();
        let journal = CreationJournal::open(store.directory().unwrap(), hub.hub).unwrap();
        assert!(journal.pending().unwrap().is_empty());
    }
    assert_ne!(
        creation.address.terminal_id.as_uuid(),
        baseline.manifest.terminal_id
    );
    workspace::no_writer(&hub, target).await;
    hub.stop().await;
}

enum Recovery {
    Unfinalized,
    AcknowledgementLost,
}
