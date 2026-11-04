use super::{CONTROL, Driver, Harness, SEARCH, SEARCH_STATUS, SELECT_SEARCH_RESULT, constants::*};
use yolo_hub_identity::TerminalAddress;
use yolo_tui_tab::TabDocument;

pub fn binding(saved: &TabDocument) -> TerminalAddress {
    let layout = &saved.tabs.active().view.layout;
    layout
        .panes()
        .iter()
        .find(|pane| pane.id == layout.focused())
        .unwrap()
        .terminal
        .unwrap()
}

pub async fn open(driver: &mut Driver, address: TerminalAddress, name: &str) {
    driver.send(SEARCH).await;
    driver.wait(|text| text.contains(SEARCH_STATUS)).await;
    let mut input = address.to_string().into_bytes();
    input.extend_from_slice(SELECT_SEARCH_RESULT);
    driver.send(&input).await;
    terminal_ready(driver, name).await;
}

pub async fn terminal_ready(driver: &mut Driver, name: &str) {
    let title = format!("{name}{CONTROL}");
    // The sidebar remains visible; only the status row proves PTY input readiness.
    driver
        .wait(|text| {
            text.lines()
                .last()
                .is_some_and(|line| line.contains(&title))
        })
        .await;
}

pub async fn exact_sessions(host: &Harness, baseline: uuid::Uuid, created: TerminalAddress) {
    let terminals = host.client().list_sessions_complete(false).await.unwrap();
    assert_eq!(terminals.len(), BASELINE_AND_CREATED);
    let ids = terminals
        .iter()
        .map(|session| session.manifest.terminal_id)
        .collect::<Vec<_>>();
    assert!(ids.contains(&baseline));
    assert!(ids.contains(&created.terminal_id.as_uuid()));
}
