#[test]
fn forms_and_stale_geometry_do_not_retarget_pointer_events() {
    let (mut state, _, _) = state();
    let before = state.tab_layout().clone();
    let view = state.presentation.panes[0];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    state.mode = Mode::ConfirmTerminate;
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert_eq!(*state.tab_layout(), before);
    state.mode = Mode::Inventory;
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert_eq!(*state.tab_layout(), before);
    assert!(incoming.try_recv().is_err());
}
