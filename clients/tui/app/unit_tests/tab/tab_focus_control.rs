use super::panes;
use crate::{Mode, network::Job};
use crossterm::event::{KeyCode, MouseButton, MouseEventKind};
use tokio::sync::mpsc;
use yolo_hub_terminal_protocol::LeaseMode;

#[test]
fn focusing_a_restored_pane_requests_safe_control_without_forwarding_the_selecting_gesture() {
    for pointer in [false, true] {
        let (mut state, first, _) = panes::state();
        let address = first.fields().address;
        let view = state.presentation.panes[0];
        let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
        if pointer {
            crate::pointer::handle(
                &mut state,
                &jobs,
                panes::mouse(
                    MouseEventKind::Down(MouseButton::Left),
                    view.inner.x,
                    view.inner.y,
                ),
            );
        } else {
            state.mode = Mode::Inventory;
            panes::repaint(&mut state);
            crate::terminal_actions::prefix(&mut state, &jobs, address.hub_id, KeyCode::Left);
        }
        assert_eq!(state.focused_terminal(), Some(address));
        assert_eq!(state.mode, Mode::Terminal);
        let Job::Acquire { request, .. } = incoming.try_recv().unwrap() else {
            panic!()
        };
        assert_eq!(request.target.address, address);
        assert_eq!(request.mode, LeaseMode::Acquire {});
        assert!(incoming.try_recv().is_err());
        assert!(state.writers.is_empty());
    }
}
