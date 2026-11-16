use super::fixtures;
use crate::{State, tabs::strip};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::{Position, Rect},
    text::Line,
};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_presentation_layout::Layout;
use yolo_tui_tab::{LocalTabView, TabViewMode};

const WIDTH: u16 = 100;
const HEIGHT: u16 = 5;
const INSET: u16 = 2;
const STRIP_HEIGHT: u16 = 1;
const COMPACT_WIDTH: u16 = 14;
const OUTSIDE_CELL: &str = "#";
const TITLES: [&str; 3] = ["Alpha", "Beta", "Gamma"];
const UNICODE_TITLE: &str = "工作 e\u{301}";
const REPLACEMENT_TITLE: &str = "Focused now";
const RENAMED_TAB: &str = "MY-BUILD";
const FIRST_NUMBER: usize = 1;

fn pages(titles: &[&str]) -> State {
    let mut state = crate::client_capability_support::state();
    for (index, title) in titles.iter().enumerate() {
        if index != 0 {
            state.tabs.create(title).unwrap();
        } else {
            state.tabs.rename(state.tabs.active().id, title).unwrap();
        }
        let mut fields = fixtures::session().fields().clone();
        fields.presentation.title = Some((*title).into());
        state
            .tabs
            .replace_active_layout(Layout::new(Some(fields.address)));
        state.tabs.update_active(LocalTabView {
            layout: state.tab_layout().clone(),
            view: TabViewMode::Terminal,
        });
        state
            .terminals
            .insert(fields.address, TerminalSummary::new(fields).unwrap());
    }
    state
}

fn render(state: &State, area: Rect) -> (Terminal<TestBackend>, Vec<strip::Tab>) {
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    let mut tabs = Vec::new();
    terminal
        .draw(|frame| {
            for cell in &mut frame.buffer_mut().content {
                cell.set_symbol(OUTSIDE_CELL);
            }
            tabs = strip::draw(frame, state, area);
        })
        .unwrap();
    (terminal, tabs)
}

fn content(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn area(width: u16) -> Rect {
    Rect::new(INSET, INSET, width, STRIP_HEIGHT)
}

#[test]
fn three_tabs_keep_catalog_order_numbers_titles_and_active_marker() {
    let state = pages(&TITLES);
    let (terminal, tabs) = render(&state, area(WIDTH - INSET));
    assert_eq!(tabs.len(), TITLES.len());
    assert_eq!(
        tabs.iter().map(|tab| tab.id).collect::<Vec<_>>(),
        state
            .tabs
            .entries()
            .iter()
            .map(|entry| entry.id)
            .collect::<Vec<_>>()
    );
    let text = content(&terminal);
    for (index, title) in TITLES.iter().enumerate() {
        assert!(text.contains(&format!("{} ◇ {title}", index + FIRST_NUMBER)));
    }
    assert!(text.contains("●3 ◇ Gamma"));
    assert_eq!(text.matches('●').count(), FIRST_NUMBER);
}

#[test]
fn active_tab_has_a_distinct_background() {
    let state = pages(&TITLES);
    let (terminal, tabs) = render(&state, area(WIDTH - INSET));
    let buffer = terminal.backend().buffer();
    let active = tabs.last().unwrap();
    let inactive = tabs.first().unwrap();
    for y in active.area.top()..active.area.bottom() {
        for x in active.area.left()..active.area.right() {
            assert_eq!(buffer[(x, y)].bg, ratatui::style::Color::Indexed(237));
        }
    }
    for y in inactive.area.top()..inactive.area.bottom() {
        for x in inactive.area.left()..inactive.area.right() {
            assert_eq!(buffer[(x, y)].bg, ratatui::style::Color::Reset);
        }
    }
}

#[test]
fn ended_tab_is_marked_in_the_strip() {
    let mut state = pages(&TITLES);
    let address = state
        .tab_layout()
        .panes()
        .into_iter()
        .find(|pane| pane.id == state.tab_layout().focused())
        .and_then(|pane| pane.terminal)
        .unwrap();
    let mut fields = state.terminals[&address].fields().clone();
    fields.runtime.as_mut().unwrap().status = yolo_hub_terminal_protocol::RuntimeStatus::Exited;
    state
        .terminals
        .insert(address, TerminalSummary::new(fields).unwrap());
    let (terminal, _) = render(&state, area(WIDTH - INSET));
    let text = content(&terminal);
    assert!(text.contains("●3 × Gamma"), "{text}");
}

#[test]
fn overflow_keeps_active_tab_visible_and_hit_targets_inside_the_strip() {
    let mut state = pages(&TITLES);
    let ids = state
        .tabs
        .entries()
        .iter()
        .map(|entry| entry.id)
        .collect::<Vec<_>>();
    for active in ids {
        state.tabs.select(active).unwrap();
        state
            .tabs
            .replace_active_layout(state.tabs.active().view.layout.clone());
        let strip_area = area(COMPACT_WIDTH);
        let (_, tabs) = render(&state, strip_area);
        assert!(tabs.iter().any(|tab| tab.id == active));
        for tab in tabs {
            assert_eq!(tab.area.intersection(strip_area), tab.area);
        }
    }
}

#[test]
fn unicode_width_and_clipping_never_touch_adjacent_tabs() {
    let state = pages(&[UNICODE_TITLE, "Next"]);
    let (_, tabs) = render(&state, area(WIDTH - INSET));
    assert_eq!(
        usize::from(tabs[0].area.width),
        Line::from(format!("  1 ◇ {UNICODE_TITLE} ")).width()
    );
    assert_eq!(tabs[1].area.x, tabs[0].area.right());
    for width in [0, 1, 2, COMPACT_WIDTH] {
        let strip_area = area(width);
        let (terminal, tabs) = render(&state, strip_area);
        assert_eq!(tabs.is_empty(), width == 0);
        for row in 0..HEIGHT {
            for col in 0..WIDTH {
                if !strip_area.contains(Position::new(col, row)) {
                    assert_eq!(
                        terminal.backend().buffer()[(col, row)].symbol(),
                        OUTSIDE_CELL
                    );
                }
            }
        }
    }
}


include!("tab_strip_rendering.rs");
