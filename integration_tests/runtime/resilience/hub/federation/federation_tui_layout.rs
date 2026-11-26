use super::{
    CONTROL_LABEL, NAME_C, POLL,
    capture::{wait_screen, wait_state},
};
use std::path::Path;
use yolo_runtime::{model::WriterLeaseGrant, worker::RuntimeRef};
#[path = "federation_tui_layout_state.rs"]
mod state;

pub async fn exercise(
    client: &yolo_client::Client,
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    workspace: &Path,
) {
    let original = state::wait(workspace, |saved| {
        let panes = saved.tabs.active().view.layout.panes();
        panes.len() == SPLIT_PANES && panes.iter().all(|pane| pane.terminal.is_some())
    })
    .await;
    wait_screen(output, screen, |text| {
        text.lines()
            .last()
            .is_some_and(|line| line.contains(CONTROL_LABEL) && !line.contains(NAME_C))
    })
    .await;
    send(client, target, grant, SHELL_OUTPUT).await;
    wait_screen(output, screen, |text| text.contains(SHELL_MARKER)).await;
    let original_position = state::label(screen.screen(), SHELL_MARKER).unwrap();
    send(client, target, grant, GROW_WIDTH).await;
    let adjusted = state::wait(workspace, |saved| {
        state::ratio(saved) < state::ratio(&original)
    })
    .await;
    wait_state(output, screen, |screen| {
        state::label(screen, SHELL_MARKER).is_some_and(|position| position.0 < original_position.0)
    })
    .await;
    let (label_x, label_y) = state::label(screen.screen(), SHELL_MARKER).unwrap();
    let (x, y) = (label_x - TITLE_INSET, label_y + BORDER_INSET);
    let press = mouse(LEFT_PRESS, x, y, PRESS_SUFFIX);
    let drag = mouse(LEFT_DRAG, x + DRAG_CELLS, y, PRESS_SUFFIX);
    let release = mouse(LEFT_PRESS, x + DRAG_CELLS, y, RELEASE_SUFFIX);
    send(client, target, grant, &press).await;
    send(client, target, grant, &drag).await;
    send(client, target, grant, &release).await;
    let dragged = state::wait(workspace, |saved| {
        state::ratio(saved) > state::ratio(&adjusted)
    })
    .await;
    assert_eq!(
        dragged.tabs.active().view.layout.panes(),
        original.tabs.active().view.layout.panes()
    );
    assert_eq!(
        dragged.tabs.active().view.layout.focused(),
        original.tabs.active().view.layout.focused()
    );
    wait_state(output, screen, |screen| {
        state::label(screen, SHELL_MARKER).is_some_and(|position| position.0 > label_x)
    })
    .await;
    send(client, target, grant, BALANCE).await;
    state::wait(workspace, |saved| {
        saved.tabs.active().view.layout == original.tabs.active().view.layout
    })
    .await;
    send(client, target, grant, FOCUS_LEFT).await;
    state::wait(workspace, |saved| {
        saved.tabs.active().view.layout.focused()
            != original.tabs.active().view.layout.focused()
    })
    .await;
    send(client, target, grant, FOCUS_RIGHT).await;
    state::wait(workspace, |saved| {
        saved.tabs.active().view.layout.focused()
            == original.tabs.active().view.layout.focused()
    })
    .await;
    send(client, target, grant, GROW_WIDTH).await;
    let final_layout = state::wait(workspace, |saved| {
        state::ratio(saved) == state::ratio(&adjusted)
    })
    .await;
    assert_eq!(
        final_layout.tabs.active().view.layout.panes(),
        original.tabs.active().view.layout.panes()
    );
    // Restore the original remote Agent pane as the selected durable binding.
    send(client, target, grant, FOCUS_LEFT).await;
    state::wait(workspace, |saved| {
        saved.tabs.active().view.layout.focused()
            != original.tabs.active().view.layout.focused()
    })
    .await;
    wait_screen(output, screen, |text| {
        text.lines()
            .last()
            .is_some_and(|line| line.contains(&format!("{NAME_C}{CONTROL_LABEL}")))
    })
    .await;
}

async fn send(
    client: &yolo_client::Client,
    target: RuntimeRef,
    grant: &WriterLeaseGrant,
    bytes: &[u8],
) {
    client
        .input(
            target,
            grant.generation,
            &grant.writer_capability,
            bytes.to_vec(),
        )
        .await
        .unwrap();
}
fn mouse(button: u16, column: u16, row: u16, suffix: char) -> Vec<u8> {
    format!(
        "{MOUSE_PREFIX}{button};{};{}{suffix}",
        column + COORDINATE_BASE,
        row + COORDINATE_BASE
    )
    .into_bytes()
}

const SPLIT_PANES: usize = 2;
const SHELL_OUTPUT: &[u8] = b"printf '\\033[2J\\033[HYOLO_LAYOUT_%s\\n' SHELL\r";
const SHELL_MARKER: &str = "YOLO_LAYOUT_SHELL";
const TITLE_INSET: u16 = 1;
const BORDER_INSET: u16 = 1;
const COORDINATE_BASE: u16 = 1;
const DRAG_CELLS: u16 = 3;
const LEFT_PRESS: u16 = 0;
const LEFT_DRAG: u16 = 32;
const MOUSE_PREFIX: &str = "\x1b[<";
const PRESS_SUFFIX: char = 'M';
const RELEASE_SUFFIX: char = 'm';
const GROW_WIDTH: &[u8] = b"\x02L";
const BALANCE: &[u8] = b"\x02=";
const FOCUS_LEFT: &[u8] = b"\x02\x1b[D";
const FOCUS_RIGHT: &[u8] = b"\x02\x1b[C";
