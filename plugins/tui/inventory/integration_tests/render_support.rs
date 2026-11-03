use ratatui::{Frame, layout::Rect};
use std::collections::BTreeMap;
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_inventory::{Inventory, render_viewport};

/// Renders one inventory frame for assertions that do not need control badges
/// or an animation phase. The production client always passes both.
pub fn render(
    frame: &mut Frame<'_>,
    inventory: &Inventory,
    terminals: &BTreeMap<TerminalAddress, TerminalSummary>,
    selected: Option<TerminalAddress>,
    filter: &str,
    area: Rect,
) {
    let _ = render_viewport(
        frame,
        inventory,
        terminals,
        selected,
        filter,
        &BTreeMap::new(),
        0,
        area,
    );
}
