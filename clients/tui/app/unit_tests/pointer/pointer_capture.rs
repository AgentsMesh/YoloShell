use super::{fixtures, panes::*};
use crate::pointer;
use crossterm::event::{MouseButton, MouseEventKind};

#[test]
fn selecting_a_pane_neither_acquires_control_nor_clicks_its_program() {
    let (mut state, first, _) = state();
    writable(&mut state, &first, ENABLE_MOUSE);
    let view = state.presentation.panes[0];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert_eq!(state.tab_layout().focused(), view.pane);
    repaint(&mut state);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Up(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert!(incoming.try_recv().is_err());
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), SGR_FIRST_CLICK);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Up(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), SGR_FIRST_RELEASE);
}

#[test]
fn release_outside_the_pane_stays_bound_to_original_terminal_and_renewal_is_not_takeover() {
    let (mut state, _, second) = state();
    writable(&mut state, &second, ENABLE_MOUSE);
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Down(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), SGR_FIRST_CLICK);
    let writer = state.writers[&second.fields().address].grant;
    let mut fields = *writer.fields();
    fields.expires_at_ms += fixtures::RENEW_MS as i64;
    state.record_grant(
        yolo_hub_terminal_protocol::LeaseGrant::new(fields).unwrap(),
        std::time::Instant::now(),
    );
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::Up(MouseButton::Left), 0, 0),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), SGR_FIRST_RELEASE);
    assert!(state.pointer.capture.is_none());
}

#[test]
fn takeover_disconnect_and_layout_replacement_fence_mouse_captures() {
    for change in 0..FENCE_CASES {
        let (mut state, _, second) = state();
        writable(&mut state, &second, ENABLE_MOUSE);
        let view = state.presentation.panes[1];
        let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
        pointer::handle(
            &mut state,
            &jobs,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                view.inner.x,
                view.inner.y,
            ),
        );
        incoming.try_recv().unwrap();
        match change {
            0 => state.record_grant(fixtures::grant(&second), std::time::Instant::now()),
            1 => state.disconnected(),
            _ => {
                state.tab_layout_mut().close(view.pane).unwrap();
                repaint(&mut state);
            }
        }
        pointer::handle(
            &mut state,
            &jobs,
            mouse(
                MouseEventKind::Up(MouseButton::Left),
                view.inner.x,
                view.inner.y,
            ),
        );
        assert!(incoming.try_recv().is_err());
        assert!(state.pointer.capture.is_none());
    }
}
const FENCE_CASES: usize = 3;

#[test]
fn a_fresh_press_recovers_after_the_outer_terminal_loses_a_release() {
    let (mut state, _, second) = state();
    writable(&mut state, &second, ENABLE_MOUSE);
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    for _ in 0..RESTARTED_PRESSES {
        pointer::handle(
            &mut state,
            &jobs,
            mouse(
                MouseEventKind::Down(MouseButton::Left),
                view.inner.x,
                view.inner.y,
            ),
        );
        assert_eq!(bytes(incoming.try_recv().unwrap()), SGR_FIRST_CLICK);
    }
    pointer::handle(
        &mut state,
        &jobs,
        mouse(
            MouseEventKind::Up(MouseButton::Left),
            view.inner.x,
            view.inner.y,
        ),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), SGR_FIRST_RELEASE);
    assert!(state.pointer.capture.is_none());
}
const RESTARTED_PRESSES: usize = 2;
