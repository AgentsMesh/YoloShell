use super::{drawing_support::*, fixtures};
use yolo_presentation_layout::Axis;
use yolo_tui_render::SYNCING_LABEL;

#[test]
fn window_and_layout_changes_show_syncing_before_resize_is_enqueued() {
    for change in [Change::Window, Change::Split, Change::Ratio] {
        let (mut state, address) = state();
        if matches!(change, Change::Ratio) {
            state
                .tab_layout_mut()
                .split(Axis::Horizontal, None)
                .unwrap();
            let source = state
                .tab_layout()
                .panes()
                .iter()
                .find(|pane| pane.terminal == Some(address))
                .unwrap()
                .id;
            state.tab_layout_mut().focus(source).unwrap();
        }
        let (_, before) = render(&state, WIDTH, HEIGHT);
        synchronize(&mut state, address, size(&before, address));
        assert!(
            render(&state, WIDTH, HEIGHT)
                .0
                .contains(&label(SOURCE_NAME, WRITING_LABEL))
        );
        let (width, height) = match change {
            Change::Window => (NEXT_WIDTH, NEXT_HEIGHT),
            Change::Split => {
                let source = state.tab_layout().focused();
                state
                    .tab_layout_mut()
                    .split(Axis::Horizontal, None)
                    .unwrap();
                state.tab_layout_mut().focus(source).unwrap();
                (WIDTH, HEIGHT)
            }
            Change::Ratio => {
                state
                    .tab_layout_mut()
                    .adjust_focused(Axis::Horizontal, true, FOCUSED_RATIO_STEP)
                    .unwrap();
                (WIDTH, HEIGHT)
            }
        };
        assert!(state.input_ready(address));
        assert!(!state.is_resize_pending(address));
        let (text, current) = render(&state, width, height);
        assert!(text.contains(&label(SOURCE_NAME, SYNCING_LABEL)));
        assert!(!text.contains(&label(SOURCE_NAME, WRITING_LABEL)));
        let desired = size(&current, address);
        state.set_last_size(address, desired);
        // Updating the desired size alone cannot turn an old snapshot into ready.
        assert!(
            render(&state, width, height)
                .0
                .contains(&label(SOURCE_NAME, SYNCING_LABEL))
        );
        synchronize(&mut state, address, desired);
        assert!(
            render(&state, width, height)
                .0
                .contains(&label(SOURCE_NAME, WRITING_LABEL))
        );
    }
}

enum Change {
    Window,
    Split,
    Ratio,
}

#[test]
fn focusing_a_new_geometry_does_not_mislabel_the_other_bound_writer() {
    let (mut state, source) = state();
    let source_tab = state.tab_layout().focused();
    let other = session(BACKGROUND_NAME);
    let background = other.fields().address;
    state.terminals.insert(background, other.clone());
    state.record_grant(fixtures::grant(&other), std::time::Instant::now());
    let other_tab = state
        .tab_layout_mut()
        .split(Axis::Horizontal, Some(background))
        .unwrap();
    let (_, initial) = render(&state, WIDTH, HEIGHT);
    synchronize(&mut state, source, size(&initial, source));
    synchronize(&mut state, background, size(&initial, background));
    let (text, changed) = render(&state, NEXT_WIDTH, NEXT_HEIGHT);
    assert!(text.contains(&label(BACKGROUND_NAME, SYNCING_LABEL)));
    assert!(!text.contains(SOURCE_NAME));
    assert!(state.presentation_ready(source, Some(background), Some(size(&changed, background))));
    state.tab_layout_mut().focus(source_tab).unwrap();
    let (text, _) = render(&state, NEXT_WIDTH, NEXT_HEIGHT);
    assert!(text.contains(&label(SOURCE_NAME, SYNCING_LABEL)));
    assert!(!text.contains(BACKGROUND_NAME));
    assert!(state.presentation_ready(background, Some(source), Some(size(&changed, source))));
    synchronize(&mut state, source, size(&changed, source));
    assert!(
        render(&state, NEXT_WIDTH, NEXT_HEIGHT)
            .0
            .contains(&label(SOURCE_NAME, WRITING_LABEL))
    );
    state.tab_layout_mut().focus(other_tab).unwrap();
    assert!(
        render(&state, NEXT_WIDTH, NEXT_HEIGHT)
            .0
            .contains(&label(BACKGROUND_NAME, SYNCING_LABEL))
    );
}

#[test]
fn matching_geometry_still_requires_resize_and_resync_completion() {
    let (mut state, address) = state();
    let (_, frame) = render(&state, WIDTH, HEIGHT);
    synchronize(&mut state, address, size(&frame, address));
    state.mark_resize_pending(address);
    assert!(
        render(&state, WIDTH, HEIGHT)
            .0
            .contains(&label(SOURCE_NAME, SYNCING_LABEL))
    );
    state.clear_all_resize_pending();
    state.mark_resync(address);
    assert!(
        render(&state, WIDTH, HEIGHT)
            .0
            .contains(&label(SOURCE_NAME, SYNCING_LABEL))
    );
    state.clear_all_resync();
    assert!(
        render(&state, WIDTH, HEIGHT)
            .0
            .contains(&label(SOURCE_NAME, WRITING_LABEL))
    );
}
