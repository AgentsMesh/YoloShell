#[test]
fn tab_name_survives_terminal_title_changes_and_unbinding() {
    let mut state = pages(&TITLES);
    let mut fields = fixtures::session().fields().clone();
    fields.presentation.title = Some(REPLACEMENT_TITLE.into());
    state
        .tabs
        .replace_active_layout(Layout::new(Some(fields.address)));
    state
        .terminals
        .insert(fields.address, TerminalSummary::new(fields).unwrap());
    let (terminal, _) = render(&state, area(WIDTH - INSET));
    assert!(!content(&terminal).contains(REPLACEMENT_TITLE));
    assert!(content(&terminal).contains(*TITLES.last().unwrap()));
    state
        .tabs
        .rename(state.tabs.active().id, RENAMED_TAB)
        .unwrap();
    let (terminal, _) = render(&state, area(WIDTH - INSET));
    assert!(content(&terminal).contains(RENAMED_TAB));
    state.tabs.replace_active_layout(Layout::default());
    let (terminal, _) = render(&state, area(WIDTH - INSET));
    assert!(content(&terminal).contains(&state.tabs.active().name));
}

#[test]
fn zero_height_strip_has_no_painted_or_clickable_tabs() {
    let state = pages(&TITLES);
    let (terminal, tabs) = render(
        &state,
        Rect {
            height: 0,
            ..area(COMPACT_WIDTH)
        },
    );
    assert!(tabs.is_empty());
    assert!(
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .all(|cell| cell.symbol() == OUTSIDE_CELL)
    );
}
