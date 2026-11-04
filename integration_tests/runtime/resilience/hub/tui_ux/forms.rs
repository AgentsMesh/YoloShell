use super::{
    constants::*,
    fixture::{Fixture, binding},
};

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_form_paste_and_directory_retry_preserve_layout_and_create_once() {
    let mut fixture = Fixture::start().await;
    let before = fixture.document(|_| true).await;
    fixture.driver.resize(SMALL_COLUMNS, SMALL_ROWS).await;
    fixture.action(INVENTORY).await;
    fixture
        .driver
        .wait(|text| text.contains(TERMINAL_PICKER_READY_SCREEN))
        .await;
    fixture.driver.send(SEARCH).await;
    fixture
        .driver
        .wait(|text| text.contains(SEARCH_STATUS))
        .await;
    fixture.driver.send(CLEAR_LINE).await;
    fixture.paste(MISSING_FILTER).await;
    fixture
        .driver
        .wait(|text| text.contains(MISSING_FILTER) && text.contains(NO_TERMINALS))
        .await;
    fixture.driver.send(CLEAR_LINE).await;
    fixture.paste(FIRST_NAME).await;
    fixture
        .driver
        .wait(|text| text.contains(ONE_TERMINAL_HEADER) && text.contains(FIRST_NAME))
        .await;
    fixture.driver.send(ENTER).await;
    fixture.driver.send(DIRECTORY).await;
    fixture
        .driver
        .wait(|text| text.contains(DIRECTORY_LABEL))
        .await;

    let parent = fixture
        .root
        .path()
        .canonicalize()
        .unwrap()
        .join(LONG_COMPONENT.repeat(LONG_COMPONENT_REPETITIONS));
    std::fs::create_dir(&parent).unwrap();
    let missing = parent.join(MISSING_DIRECTORY);
    let valid = parent.join(VALID_DIRECTORY);
    std::fs::create_dir(&valid).unwrap();
    fixture.driver.send(CLEAR_LINE).await;
    fixture.paste(missing.to_str().unwrap()).await;
    fixture
        .driver
        .wait(|text| text.contains(DIRECTORY_LABEL) && text.contains(MISSING_DIRECTORY))
        .await;
    let (_, cursor_column) = fixture.driver.screen().cursor_position();
    assert!(cursor_column < SMALL_COLUMNS);
    fixture.driver.send(ENTER).await;
    fixture
        .driver
        .wait(|text| text.contains(INVALID_DIRECTORY) && text.contains(DIRECTORY_LABEL))
        .await;
    let rejected = fixture
        .document(|saved| {
            saved.tabs.entries().len() == before.tabs.entries().len()
                && saved.tabs.active().id == before.tabs.active().id
                && saved.tabs.active().view.layout == before.tabs.active().view.layout
        })
        .await;
    assert_eq!(binding(&rejected), Some(fixture.target.address));
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

    fixture.driver.send(CLEAR_LINE).await;
    fixture.paste(valid.to_str().unwrap()).await;
    fixture
        .driver
        .wait(|text| text.contains(VALID_DIRECTORY) && !text.contains(INVALID_DIRECTORY))
        .await;
    fixture.driver.send(ENTER).await;
    let created = fixture
        .document(|saved| {
            saved.tabs.entries().len() == TWO_TABS
                && binding(saved).is_some_and(|address| address != fixture.target.address)
        })
        .await;
    let address = binding(&created).unwrap();
    let terminal = fixture
        .host
        .client()
        .get_session(address.terminal_id.as_uuid())
        .await
        .unwrap();
    fixture.host.track_view(&terminal);
    assert_eq!(terminal.manifest.cwd, valid);
    fixture.marker().await;
    assert_eq!(
        fixture
            .host
            .client()
            .list_sessions_complete(false)
            .await
            .unwrap()
            .len(),
        TWO_TERMINALS
    );
    fixture.baseline_is_live().await;
    // A local field paste must not leave an invisible draft in the original PTY.
    fixture.driver.open(FIRST_NAME, CONTROL).await;
    fixture.marker().await;
    fixture.finish().await;
}
