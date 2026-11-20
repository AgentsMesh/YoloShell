use super::{RunningNode, deadline};
use std::path::Path;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_tui_tab::{TAB_FILE, TabDocument};

pub async fn track_created(origin: &crate::Harness, path: &Path, entry: HubId) {
    let saved = tokio::time::timeout_at(deadline(), async {
        loop {
            let saved: TabDocument =
                yolo_runtime::paths::read_json(&path.join(TAB_FILE)).unwrap();
            saved.validate(entry).unwrap();
            let panes = saved.tabs.active().view.layout.panes();
            if panes.len() == SPLIT_PANES && panes.iter().all(|pane| pane.terminal.is_some()) {
                break saved;
            }
            tokio::time::sleep(super::POLL).await;
        }
    })
    .await
    .unwrap();
    for address in saved
        .tabs
        .active()
        .view        .layout
        .panes()
        .into_iter()
        .filter_map(|pane| pane.terminal)
    {
        let view = origin
            .client()
            .get_session(address.terminal_id.as_uuid())
            .await
            .unwrap();
        origin.track_view(&view);
    }
}

pub fn assert_restored(restored: &TabDocument, saved: &TabDocument) {
    assert_eq!(restored.entry_hub, saved.entry_hub);
    assert_eq!(restored.schema_version, saved.schema_version);
    assert_eq!(restored.tabs, saved.tabs);
    assert!(restored.revision >= saved.revision);
}

pub fn checkpoint(path: &Path, entry: HubId) -> TabDocument {
    let saved: TabDocument =
        yolo_runtime::paths::read_json(&path.join(TAB_FILE)).unwrap();
    saved.validate(entry).unwrap();
    let panes = saved.tabs.active().view.layout.panes();
    assert_eq!(panes.len(), SPLIT_PANES);
    assert!(
        panes
            .iter()
            .any(|pane| { pane.terminal.is_some_and(|terminal| terminal.hub_id != entry) })
    );
    let focused = panes
        .iter()
        .find(|pane| pane.id == saved.tabs.active().view.layout.focused())
        .unwrap();
    assert!(focused.terminal.is_some());
    assert!(panes.iter().all(|pane| pane.terminal.is_some()));
    saved
}

pub async fn assert_passive(node: &RunningNode, workspace: &TabDocument) {
    let mut client = node.client().await;
    for address in workspace
        .tabs
        .active()
        .view        .layout
        .panes()
        .iter()
        .filter_map(|pane| pane.terminal)
    {
        let Reply::Terminal { terminal: session } = client
            .call(address.hub_id, Command::Get { address }, deadline())
            .await
            .unwrap()
        else {
            panic!()
        };
        let target = session.fields().runtime.unwrap().target;
        let reply = client
            .call(address.hub_id, Command::ObserveLease { target }, deadline())
            .await
            .unwrap();
        assert!(matches!(reply, Reply::LeaseObservation { lease: None }));
    }
}

const SPLIT_PANES: usize = 2;
