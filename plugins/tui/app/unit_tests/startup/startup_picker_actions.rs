#[test]
fn terminal_picker_escape_cancels_the_pending_reopen() {
    let mut fixture = Fixture::new(StartupAction::TerminalPicker);
    startup::user_event(
        &mut fixture.state,
        &Event::Key(KeyEvent::from(KeyCode::Esc)),
    );
    fixture.state.terminal_picker = None;
    fixture.ready();
    fixture.apply();
    assert!(fixture.state.terminal_picker.is_none());
    fixture.no_jobs();
}

#[test]
fn explicit_session_list_overrides_restored_terminal_view_without_rebinding_or_acquiring() {
    let mut f = Fixture::automatic();
    let shell = session(f.hub, RuntimeStatus::Running);
    f.state.replace_terminals(vec![shell.clone()]);
    f.state.bind_focused(Some(shell.fields().address)).unwrap();
    f.state.mode = Mode::Terminal;
    let layout = f.state.tab_layout().clone();
    startup::install(&mut f.state, f.hub, StartupAction::Inventory);
    f.ready();
    f.apply();
    f.no_jobs();
    assert_eq!(f.state.mode, Mode::Inventory);
    assert_eq!(*f.state.tab_layout(), layout);
    assert!(f.state.writers.is_empty());
}
