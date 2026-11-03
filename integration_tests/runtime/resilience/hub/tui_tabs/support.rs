use super::constants::*;
use crate::Harness;
use crate::hub::{support::deadline, tui_shell_first::driver::Driver};
use std::{collections::BTreeSet, path::Path};
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_runtime::model::TerminalView;
use yolo_tui_tab::{TAB_FILE, TabDocument, TabCatalog};

pub(super) async fn document(
    directory: &Path,
    hub: HubId,
    matches: impl Fn(&TabDocument) -> bool,
) -> TabDocument {
    let mut observed = None;
    let result = tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(saved) =
                yolo_runtime::paths::read_json::<TabDocument>(&directory.join(TAB_FILE))
            {
                saved.validate(hub).unwrap();
                if matches(&saved) {
                    return saved;
                }
                observed = Some(saved);
            }
            tokio::time::sleep(POLL).await;
        }
    })
    .await;
    result.unwrap_or_else(|error| panic!("tab document did not converge: {error}; {observed:?}"))
}

pub(super) fn bindings(saved: &TabDocument) -> BTreeSet<TerminalAddress> {
    saved
        .tabs
        .entries()
        .iter()
        .flat_map(|tab| tab.view.layout.panes())
        .filter_map(|pane| pane.terminal)
        .collect()
}

pub(super) fn focused(saved: &TabDocument) -> TerminalAddress {
    let layout = &saved.tabs.active().view.layout;
    layout
        .panes()
        .into_iter()
        .find(|pane| pane.id == layout.focused())
        .unwrap()
        .terminal
        .unwrap()
}

pub(super) fn bound_panes(saved: &TabDocument, tabs: usize, panes: usize) -> bool {
    let layout = &saved.tabs.active().view.layout;
    saved.tabs.entries().len() == tabs
        && layout.panes().len() == panes
        && layout.panes().iter().all(|pane| pane.terminal.is_some())
}

pub(super) async fn runtime(host: &Harness, address: TerminalAddress) -> TerminalView {
    let view = host
        .client()
        .get_session(address.terminal_id.as_uuid())
        .await
        .unwrap();
    host.track_view(&view);
    view
}

pub(super) async fn writable(driver: &mut Driver) {
    driver
        .wait(|text| {
            text.lines()
                .last()
                .is_some_and(|line| line.contains(CONTROL))
        })
        .await;
}

pub(super) async fn action(driver: &mut Driver, keys: &[u8]) {
    let command = keys.strip_prefix(PREFIX).unwrap();
    driver.send(PREFIX).await;
    // Consume the prefix frame so a cached healthy status from the previous
    // pane cannot satisfy the next pane's input-readiness assertion.
    driver.wait(|text| text.contains(PREFIX_HINT)).await;
    driver.send(command).await;
}

pub(super) async fn marker(driver: &mut Driver) {
    writable(driver).await;
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let expected = format!("{MARKER_PREFIX}{nonce}");
    let command = format!("printf '{MARKER_PREFIX}%s\\n' '{nonce}'\r");
    // The concatenated output is absent from the input, so shell echo cannot pass.
    driver.send(command.as_bytes()).await;
    driver.wait(|text| text.contains(&expected)).await;
}

pub(super) fn same_layouts(before: &TabCatalog, after: &TabCatalog) {
    assert_eq!(before.active().id, after.active().id);
    assert_eq!(before.entries().len(), after.entries().len());
    for (before, after) in before.entries().iter().zip(after.entries()) {
        assert_eq!(before.id, after.id);
        assert_eq!(before.name, after.name);
        assert_eq!(before.view.layout, after.view.layout);
    }
}
