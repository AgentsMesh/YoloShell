use crate::{Mode, State};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
use yolo_presentation_layout::{
    Axis, Layout, MAX_RATIO, MIN_RATIO, RATIO_SCALE, SplitGeometry, SplitRatio,
};

pub struct Drag {
    split: SplitGeometry,
    expected: Layout,
    mode: Mode,
    bounds: ratatui::layout::Rect,
    origin: u16,
}

pub fn handle(state: &mut State, event: MouseEvent) -> bool {
    if state.pane_view.zoomed {
        state.pointer.drag = None;
        return false;
    }
    if let Some(mut drag) = state.pointer.drag.take() {
        if drag.expected != *state.tab_layout()
            || drag.mode != state.mode
            || drag.bounds != state.presentation.bounds
        {
            return true;
        }
        if matches!(
            event.kind,
            MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Up(MouseButton::Left)
        ) {
            let (position, span) = match drag.split.axis {
                Axis::Horizontal => (event.column, drag.split.rect.width),
                Axis::Vertical => (event.row, drag.split.rect.height),
            };
            if span != 0 {
                let delta = (i64::from(position) - i64::from(drag.origin)) * i64::from(RATIO_SCALE)
                    / i64::from(span);
                let ratio = (i64::from(u16::from(drag.split.ratio)) + delta)
                    .clamp(i64::from(MIN_RATIO), i64::from(MAX_RATIO))
                    as u16;
                let result = SplitRatio::try_from(ratio)
                    .and_then(|ratio| state.tab_layout_mut().resize_split(&drag.split.path, ratio));
                if let Err(error) = result {
                    state.status = error.to_string();
                    return true;
                }
                drag.expected = state.tab_layout().clone();
            }
            if matches!(event.kind, MouseEventKind::Drag(_)) {
                state.pointer.drag = Some(drag);
            }
        }
        return true;
    }
    if event.kind != MouseEventKind::Down(MouseButton::Left)
        || !state.presentation.current(&state.tab_layout(), state.mode)
    {
        return false;
    }
    let Ok(splits) = state
        .tab_layout()
        .split_geometry(state.presentation.geometry_bounds())
    else {
        return false;
    };
    let Some(split) = splits
        .into_iter()
        .filter(|split| hit(split, event, state.mode == Mode::Terminal))
        .max_by_key(|split| split.path.len())
    else {
        return false;
    };
    state.pointer.capture = None;
    let origin = match split.axis {
        Axis::Horizontal => event.column,
        Axis::Vertical => event.row,
    };
    state.pointer.drag = Some(Drag {
        split,
        expected: state.tab_layout().clone(),
        mode: state.mode,
        bounds: state.presentation.bounds,
        origin,
    });
    true
}

fn hit(split: &SplitGeometry, event: MouseEvent, compact: bool) -> bool {
    let rect = ratatui::layout::Rect::new(
        split.rect.x,
        split.rect.y,
        split.rect.width,
        split.rect.height,
    );
    if !crate::pointer::contains(rect, event.column, event.row) {
        return false;
    }
    let (position, start, span) = match split.axis {
        Axis::Horizontal => (event.column, split.rect.x, split.rect.width),
        Axis::Vertical => (event.row, split.rect.y, split.rect.height),
    };
    split.boundary > start
        && u32::from(split.boundary) < u32::from(start) + u32::from(span)
        && (position == split.boundary
            || (!compact
                && position.checked_add(crate::constants::PANE_BORDER_CELLS)
                    == Some(split.boundary)))
}
