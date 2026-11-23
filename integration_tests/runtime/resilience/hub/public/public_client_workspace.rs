use super::{deadline, driver::Driver, fixture::Fixture};
use yolo_hub_identity::TerminalAddress;
use yolo_tui_tab::{TAB_FILE, TabDocument};

pub fn binding(saved: &TabDocument) -> Option<TerminalAddress> {
    let layout = &saved.tabs.active().view.layout;
    layout
        .panes()
        .iter()
        .find(|pane| pane.id == layout.focused())
        .unwrap()
        .terminal
}

pub async fn bound(driver: &Driver, fixture: &Fixture) -> TabDocument {
    tokio::time::timeout_at(deadline(), async {
        loop {
            if let Ok(saved) = yolo_runtime::paths::read_json::<TabDocument>(
                &driver.workspace.path().join(TAB_FILE),
            ) {
                saved.validate(fixture.hub).unwrap();
                if binding(&saved).is_some() {
                    return saved;
                }
            }
            tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
        }
    })
    .await
    .expect("remote TUI never persisted its shell binding")
}
