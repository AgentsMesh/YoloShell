#[test]
fn latest_provider_ai_title_is_selected_only_for_the_matching_session() {
    let values = vec![
        json!({
            (constants::RECORD_TYPE_KEY): constants::AI_TITLE_RECORD,
            (constants::SESSION_ID_KEY): SESSION_ID,
            (constants::AI_TITLE_KEY): "first title",
        }),
        json!({
            (constants::RECORD_TYPE_KEY): constants::AI_TITLE_RECORD,
            (constants::SESSION_ID_KEY): SESSION_ID,
            (constants::AI_TITLE_KEY): "最新标题",
        }),
        json!({
            (constants::RECORD_TYPE_KEY): constants::AI_TITLE_RECORD,
            (constants::SESSION_ID_KEY): OTHER_SESSION_ID,
            (constants::AI_TITLE_KEY): "wrong session",
        }),
    ];
    assert_eq!(
        display_title(&values, SESSION_ID).as_deref(),
        Some("最新标题")
    );
    assert_eq!(
        display_title(&values, OTHER_SESSION_ID).as_deref(),
        Some("wrong session")
    );
    assert_eq!(display_title(&values, "missing"), None);
}
