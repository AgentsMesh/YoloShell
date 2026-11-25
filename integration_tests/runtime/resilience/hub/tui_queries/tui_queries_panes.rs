use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_background_pty_queries_reply_to_source_terminal_instead_of_keyboard_focus() {
    let host = Harness::start().await;
    let root = tempfile::tempdir_in(SOCKET_TEST_ROOT).unwrap();
    let hub = RunningHub::start(
        &host,
        &root.path().canonicalize().unwrap().join(STATE_DIRECTORY),
    )
    .await;
    let mut first = Source::start(&host, hub.hub, FIRST_NAME).await;
    let mut second = Source::start(&host, hub.hub, SECOND_NAME).await;
    let driver_host = Harness::start().await;
    let mut driver = Driver::start(&driver_host, &hub, tempfile::tempdir().unwrap()).await;
    driver.open(first.name, CONTROL).await;
    driver.open(second.name, CONTROL).await;
    driver
        .focused(hub.hub, second.target.address, second.name)
        .await;
    eprintln!("terminal query phase: second pane focused baseline");
    second
        .expect(SECOND_CURSOR_QUERY, SECOND_CURSOR_REPLY)
        .await;
    driver.send(PREVIOUS_TAB).await;
    driver
        .focused(hub.hub, first.target.address, first.name)
        .await;
    driver.access(first.name, CONTROL).await;
    eprintln!("terminal query phase: first pane focused baseline");
    first.expect(FIRST_CURSOR_QUERY, FIRST_CURSOR_REPLY).await;
    driver.send(NEXT_TAB).await;
    driver
        .focused(hub.hub, second.target.address, second.name)
        .await;
    driver.access(second.name, CONTROL).await;
    // First has been resized while focused; now it issues queries in the background.
    let first_expected = [FIRST_CURSOR_REPLY, FIRST_CURSOR_REPLY].concat();
    eprintln!("terminal query phase: first pane background, second pane focused");
    first.expect(FIRST_CURSOR_QUERY, &first_expected).await;
    assert_eq!(second.emit(EMPTY_OUTPUT).await, SECOND_CURSOR_REPLY);
    let second_expected = [SECOND_CURSOR_REPLY, SECOND_CURSOR_REPLY].concat();
    eprintln!("terminal query phase: second pane focused follow-up");
    second.expect(SECOND_CURSOR_QUERY, &second_expected).await;
    assert_eq!(first.emit(EMPTY_OUTPUT).await, first_expected);
    driver.finish().await;
    hub.stop().await;
}
