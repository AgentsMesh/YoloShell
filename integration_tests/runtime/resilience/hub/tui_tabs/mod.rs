mod constants;
mod support;
use crate::Harness;
use crate::hub::{
    support::{RunningHub, SOCKET_TEST_ROOT, STATE_DIRECTORY},
    tui_shell_first::driver::Driver,
    tui_workspaces::support::runtime_unchanged,
};
use constants::*;
use support::*;
use yolo_hub_identity::{RuntimeEpoch, RuntimeTarget};

#[tokio::test(flavor = "multi_thread")]
async fn real_cli_tabs_create_split_reopen_and_restore_without_replacing_live_ptys() {
    let host = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let directory = root.path().canonicalize().unwrap();
    let hub = RunningHub::start(&host, &directory.join(STATE_DIRECTORY)).await;
    let client_state = directory.join(CLIENT_DIRECTORY);
    let driver_host = Harness::start().await;
    let mut driver = Driver::new(&driver_host).await;
    driver.launch(&hub, &client_state, AUTO_COMMAND).await;
    let first = document(&client_state, hub.hub, |saved| {
        bound_panes(saved, ONE_TAB, ONE_PANE)
    })
    .await;
    let first_tab = first.tabs.active().id;
    let first_terminal = focused(&first);
    let first_runtime = runtime(&host, first_terminal).await;
    marker(&mut driver).await;

    action(&mut driver, NEW_TAB).await;
    let second = document(&client_state, hub.hub, |saved| {
        bound_panes(saved, TWO_TABS, ONE_PANE) && saved.tabs.active().id != first_tab
    })
    .await;
    let second_tab = second.tabs.active().id;
    assert_eq!(
        second
            .tabs
            .entries()
            .iter()
            .find(|tab| tab.id == first_tab)
            .unwrap()
            .view
            .layout,
        first.tabs.active().view.layout,
    );
    let second_terminal = focused(&second);
    assert_ne!(second_terminal, first_terminal);
    marker(&mut driver).await;

    action(&mut driver, SPLIT_RIGHT).await;
    let split = document(&client_state, hub.hub, |saved| {
        bound_panes(saved, TWO_TABS, TWO_PANES)
    })
    .await;
    assert_eq!(split.tabs.active().id, second_tab);
    assert!(bindings(&split).is_superset(&bindings(&second)));
    marker(&mut driver).await;
    action(&mut driver, SPLIT_DOWN).await;
    let expanded = document(&client_state, hub.hub, |saved| {
        bound_panes(saved, TWO_TABS, THREE_PANES)
    })
    .await;
    assert_eq!(expanded.tabs.active().id, second_tab);
    assert!(bindings(&expanded).is_superset(&bindings(&split)));
    marker(&mut driver).await;

    let mut runtimes = Vec::new();
    for address in bindings(&expanded) {
        runtimes.push((address, runtime(&host, address).await));
    }
    assert_eq!(runtimes.len(), FOUR_TERMINALS);
    assert_eq!(
        host.client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .len(),
        FOUR_TERMINALS
    );
    action(&mut driver, PREVIOUS_TAB).await;
    document(&client_state, hub.hub, |saved| {
        saved.tabs.active().id == first_tab
    })
    .await;
    marker(&mut driver).await;
    action(&mut driver, NEXT_TAB).await;
    document(&client_state, hub.hub, |saved| {
        saved.tabs.active().id == second_tab
            && saved.tabs.active().view.layout
                == expanded.tabs.active().view.layout
    })
    .await;
    marker(&mut driver).await;

    action(&mut driver, TERMINAL_PICKER).await;
    driver.wait(|text| text.contains(TERMINAL_PICKER_HEADER)).await;
    driver.send(SEARCH).await;
    driver.wait(|text| text.contains(SEARCH_LABEL)).await;
    driver.send(first_terminal.to_string().as_bytes()).await;
    driver.send(OPEN_SEARCH_RESULT).await;
    let reopened = document(&client_state, hub.hub, |saved| {
        saved.tabs.active().id == first_tab && focused(saved) == first_terminal
    })
    .await;
    assert_eq!(bindings(&reopened), bindings(&expanded));
    assert_eq!(reopened.tabs.entries().len(), TWO_TABS);
    marker(&mut driver).await;
    action(&mut driver, SIDEBAR).await;
    driver
        .wait(|text| text.contains(AGENTS_HEADER) && text.contains(HUBS_LABEL))
        .await;
    assert!(!driver.screen().contents().contains(RETIRED_TERMINAL_PICKER_LABEL));
    action(&mut driver, DETACH).await;
    driver.returned().await;
    let saved = document(&client_state, hub.hub, |_| true).await;

    driver.launch(&hub, &client_state, ATTACH_COMMAND).await;
    let restored = document(&client_state, hub.hub, |saved| {
        bound_panes(saved, TWO_TABS, ONE_PANE)
    })
    .await;
    same_layouts(&saved.tabs, &restored.tabs);
    marker(&mut driver).await;
    for (address, before) in &runtimes {
        runtime_unchanged(
            &host,
            RuntimeTarget {
                address: *address,
                runtime_epoch: RuntimeEpoch::from_uuid(
                    before.runtime.as_ref().unwrap().runtime_epoch,
                )
                .unwrap(),
            },
            before,
        )
        .await;
    }
    assert_eq!(
        crate::runtime_ref_from_view(&runtime(&host, first_terminal).await),
        crate::runtime_ref_from_view(&first_runtime),
    );
    assert_eq!(
        host.client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .len(),
        FOUR_TERMINALS
    );
    action(&mut driver, DETACH).await;
    driver.returned().await;
    driver.close().await;
    hub.stop().await;
}
