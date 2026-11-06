use super::{constants::*, deadline, driver::Driver, fixture::Fixture, shell};
use crate::{Harness, hub::support::input};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::LeaseMode;

#[tokio::test(flavor = "multi_thread")]
async fn remote_tui_observer_views_live_output_without_writer_or_lifecycle_grants() {
    let fixture = Fixture::start(&OBSERVER_RIGHTS).await;
    let (terminal, original) = fixture.create().await;
    let target = terminal.fields().runtime.unwrap().target;
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &fixture, tempfile::tempdir().unwrap()).await;
    driver.send(INVENTORY).await;
    driver
        .wait(|text| text.contains(TERMINAL_PICKER_READY_SCREEN))
        .await;
    driver.send(SEARCH).await;
    driver.wait(|text| text.contains(SEARCH_STATUS)).await;
    let mut selection = target.address.to_string().into_bytes();
    selection.extend_from_slice(SELECT_SEARCH_RESULT);
    driver.send(&selection).await;
    driver.wait(|text| text.contains(READ_ONLY)).await;
    fixture.no_writer(target).await;

    // A fully authorized local client provides a causal output fence. The viewer
    // sees its output but never becomes a writer or creates a replacement PTY.
    let mut local = fixture.process.client().await;
    let Reply::Lease { grant } = local
        .call(
            fixture.hub,
            fixture.lease_command(target, LeaseMode::Acquire {}),
            deadline(),
        )
        .await
        .unwrap()
    else {
        panic!("local witness writer is missing")
    };
    let (command, marker) = shell::marker();
    local
        .call(fixture.hub, input(grant, &command), deadline())
        .await
        .unwrap();
    driver
        .wait(|text| text.contains(&marker) && text.contains(READ_ONLY))
        .await;
    local
        .call(fixture.hub, Command::Release { grant }, deadline())
        .await
        .unwrap();
    driver.finish().await;
    fixture.no_writer(target).await;
    fixture.custody(&original).await;
    assert_eq!(
        fixture
            .host
            .client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .len(),
        SHELL_COUNT
    );
    fixture.stop().await;
}
