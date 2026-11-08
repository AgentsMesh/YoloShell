use super::{constants::*, fixture::Fixture};

#[tokio::test(flavor = "multi_thread")]
async fn real_tui_help_scrolls_at_eighty_columns_and_keeps_its_exit_hint_visible() {
    let mut fixture = Fixture::start().await;
    fixture.driver.resize(SMALL_COLUMNS, SMALL_ROWS).await;
    fixture.action(HELP).await;
    fixture
        .driver
        .wait(|text| {
            text.contains(HELP_TITLE)
                && text
                    .lines()
                    .last()
                    .is_some_and(|line| line.contains(HELP_CLOSE))
        })
        .await;
    assert!(fixture.driver.screen().contents().contains(HELP_TOP));
    assert!(!fixture.driver.screen().contents().contains(HELP_LAST));
    fixture.driver.send(END).await;
    fixture
        .driver
        .wait(|text| {
            text.contains(HELP_LAST)
                && text
                    .lines()
                    .last()
                    .is_some_and(|line| line.contains(HELP_CLOSE))
        })
        .await;
    fixture.driver.send(PAGE_UP).await;
    fixture
        .driver
        .wait(|text| {
            text.contains(HELP_TOP)
                && text
                    .lines()
                    .last()
                    .is_some_and(|line| line.contains(HELP_CLOSE))
        })
        .await;
    fixture.driver.send(CLOSE_HELP).await;
    fixture.writable().await;
    assert!(!fixture.driver.screen().contents().contains(HELP_TITLE));
    fixture.marker().await;
    fixture.baseline_is_live().await;
    fixture.finish().await;
}
