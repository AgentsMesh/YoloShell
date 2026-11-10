use super::{constants::*, support::*, *};

pub async fn exercise(
    driver: &mut Driver,
    hub: &RunningHub,
    host: &Harness,
    first: &mut Source,
    second: &Source,
) {
    driver.open(first.name, CONTROL).await;
    driver.send(SPLIT).await;
    let split = document(driver, hub, |saved| {
        let panes = saved.tabs.active().view.layout.panes();
        panes.len() == PANE_COUNT && panes.iter().all(|pane| pane.terminal.is_some())
    })
    .await;
    let created = split
        .tabs
        .active()
        .view        .layout
        .panes()
        .into_iter()
        .filter_map(|pane| pane.terminal)
        .find(|address| *address != first.target.address)
        .unwrap();
    let created_before = terminal(hub, created).await;
    host.track_view(
        &host
            .client()
            .get_session(created.terminal_id.as_uuid())
            .await
            .unwrap(),
    );
    driver.send(FOCUS_OTHER).await;
    driver
        .focused(hub.hub, first.target.address, first.name)
        .await;
    driver.send(SIDEBAR).await;
    driver.wait(|text| text.contains(AGENTS_HEADER)).await;
    // Sidebar tabs contain only Agent runs and Hubs.
    click(driver, HUBS_TAB).await;
    driver.wait(|text| text.contains(HUBS_HEADER)).await;
    click(driver, AGENTS_TAB).await;
    driver.wait(|text| text.contains(AGENTS_HEADER)).await;
    driver.send(INVENTORY).await;
    driver.clear_filter().await;
    driver
        .wait(|text| text.contains(TERMINAL_PICKER_READY_SCREEN) && text.contains(second.name))
        .await;
    assert!(lease(hub, second.target).await.is_none());
    click(driver, second.name).await;
    driver
        .wait(|text| {
            text.contains(SELECTED_LABEL)
                && text.contains(&format!("{SELECTION_MARKER}{}", second.name))
        })
        .await;
    assert!(lease(hub, second.target).await.is_none());
    driver.send(ESCAPE).await;
    driver.access(first.name, CONTROL).await;
    let original = document(driver, hub, |saved| {
        saved.tabs.active().view.layout.panes().len() == PANE_COUNT
    })
    .await;
    let original_layout = original.tabs.active().view.layout.clone();
    let original_size = source_size(hub, first).await;
    driver.send(ZOOM).await;
    let zoomed =
        yolo_hub_terminal_protocol::TerminalSize::new(COLUMNS - SIDEBAR_COLUMNS, ROWS - CHROME_ROWS)
            .unwrap();
    settled_screen(driver, hub, first, zoomed).await;
    assert_eq!(
        document(driver, hub, |_| true)
            .await
            .tabs
            .active()
            .view            .layout,
        original_layout
    );
    driver.send(ZOOM).await;
    settled_screen(driver, hub, first, original_size).await;
    let original_panes = original_layout
        .panes()
        .iter()
        .map(|pane| pane.id)
        .collect::<Vec<_>>();
    driver.send(SWAP).await;
    document(driver, hub, |saved| {
        saved
            .tabs
            .active()
            .view            .layout
            .panes()
            .iter()
            .map(|pane| pane.id)
            .collect::<Vec<_>>()
            == original_panes.iter().rev().copied().collect::<Vec<_>>()
    })
    .await;
    driver.send(FOCUS_OTHER).await;
    driver.send(CLOSE_PANE).await;
    document(driver, hub, |saved| {
        saved.tabs.active().view.layout.panes().len() == SINGLE_PANE
    })
    .await;
    driver
        .focused(hub.hub, first.target.address, first.name)
        .await;
    no_writer(hub, second.target).await;
    let created_after = terminal(hub, created).await;
    assert_eq!(
        created_before.fields().runtime.unwrap().target,
        created_after.fields().runtime.unwrap().target
    );
    assert_eq!(
        created_after.fields().runtime.unwrap().status,
        yolo_hub_terminal_protocol::RuntimeStatus::Running
    );
    no_writer(hub, created_after.fields().runtime.unwrap().target).await;
}

async fn terminal(
    hub: &RunningHub,
    address: yolo_hub_identity::TerminalAddress,
) -> yolo_hub_terminal_protocol::TerminalSummary {
    let yolo_hub_rpc_protocol::Reply::Terminal { terminal: session } = hub
        .client()
        .await
        .call(
            address.hub_id,
            yolo_hub_rpc_protocol::Command::Get { address },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("terminal metadata required");
    };
    session
}

async fn source_size(hub: &RunningHub, source: &Source) -> yolo_hub_terminal_protocol::TerminalSize {
    let yolo_hub_rpc_protocol::Reply::Terminal { terminal: session } = hub
        .client()
        .await
        .call(
            hub.hub,
            yolo_hub_rpc_protocol::Command::Get {
                address: source.target.address,
            },
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("session metadata required for viewport size")
    };
    session.fields().runtime.unwrap().size.unwrap()
}

async fn settled_screen(
    driver: &mut Driver,
    hub: &RunningHub,
    source: &mut Source,
    size: yolo_hub_terminal_protocol::TerminalSize,
) {
    tokio::time::timeout_at(deadline(), async {
        while source_size(hub, source).await != size {
            tokio::time::sleep(yolo_terminal_query_fixture::constants::POLL).await;
        }
    })
    .await
    .unwrap();
    let marker = uuid::Uuid::new_v4().simple().to_string();
    assert!(
        source
            .emit(format!("{RESET_SCREEN}{marker}").as_bytes())
            .await
            .is_empty()
    );
    let title = format!("{}{CONTROL}", source.name);
    driver
        .wait(|text| text.contains(&marker) && text.contains(&title))
        .await;
}

const RESET_SCREEN: &str = "\x1b[2J\x1b[1;1H";
