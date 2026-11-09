use super::{
    constants::*,
    fixture::{Fixture, binding, empty},
};

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_tab_rename_and_both_last_view_close_paths_keep_the_original_pty() {
    let mut fixture = Fixture::start().await;
    let original = fixture.document(|_| true).await;
    fixture.action(MANAGER).await;
    fixture
        .driver
        .wait(|text| text.lines().next().map(str::trim_end) == Some(MANAGER_TITLE))
        .await;
    fixture.driver.send(RENAME).await;
    fixture
        .driver
        .wait(|text| text.contains(RENAME_PROMPT))
        .await;
    fixture.driver.send(CLEAR_LINE).await;
    fixture.paste(RENAMED_TAB).await;
    fixture.driver.wait(|text| text.contains(RENAMED_TAB)).await;
    fixture.driver.send(ENTER).await;
    fixture
        .driver
        .wait(|text| {
            text.lines()
                .next()
                .is_some_and(|line| line.contains(RENAMED_TAB))
        })
        .await;
    fixture
        .document(|saved| saved.tabs.active().name == RENAMED_TAB)
        .await;
    fixture.marker().await;

    fixture.action(MANAGER).await;
    fixture
        .driver
        .wait(|text| text.lines().next().map(str::trim_end) == Some(MANAGER_TITLE))
        .await;
    fixture.driver.send(REMOVE).await;
    fixture
        .driver
        .wait(|text| text.contains(REMOVE_PROMPT))
        .await;
    fixture.driver.send(ENTER).await;
    fixture.driver.wait(|text| text.contains(EMPTY_AGENT_INVENTORY)).await;
    let managed = fixture.document(empty).await;
    assert_eq!(managed.tabs.active().id, original.tabs.active().id);
    assert_eq!(managed.tabs.active().name, RENAMED_TAB);
    fixture.baseline_is_live().await;

    fixture.driver.open(FIRST_NAME, CONTROL).await;
    fixture
        .document(|saved| binding(saved) == Some(fixture.target.address))
        .await;
    fixture.marker().await;
    fixture.action(CLOSE_TAB).await;
    fixture.driver.wait(|text| text.contains(EMPTY_AGENT_INVENTORY)).await;
    let closed = fixture.document(empty).await;
    assert_eq!(closed.tabs.active().id, managed.tabs.active().id);
    assert_eq!(closed.tabs.active().name, managed.tabs.active().name);
    assert_eq!(
        fixture
            .host
            .client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .len(),
        ONE_TERMINAL
    );
    fixture.baseline_is_live().await;
    fixture.finish().await;
}
