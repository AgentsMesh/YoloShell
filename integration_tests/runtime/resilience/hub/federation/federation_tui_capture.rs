use super::{RunningNode, deadline};
use crate::hub::tui_command::TERMINALS_COMMAND;
use std::path::Path;
use tokio::time::timeout_at;
use yolo_runtime::protocol::{event, from_value};
pub(super) async fn wait_screen(
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    ready: impl Fn(&str) -> bool,
) {
    wait_state(output, screen, |screen| ready(&screen.contents())).await;
}
pub(super) async fn try_wait_screen(
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    ready: impl Fn(&str) -> bool,
) -> Result<(), String> {
    try_wait_state(output, screen, |screen| ready(&screen.contents())).await
}
pub(super) async fn wait_state(
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    ready: impl Fn(&vt100::Screen) -> bool,
) {
    if let Err(error) = try_wait_state(output, screen, ready).await {
        panic!("{error}");
    }
}
async fn try_wait_state(
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    ready: impl Fn(&vt100::Screen) -> bool,
) -> Result<(), String> {
    try_wait_state_until(output, screen, deadline(), ready).await
}

pub(super) async fn try_wait_screen_until(
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    until: tokio::time::Instant,
    ready: impl Fn(&str) -> bool,
) -> Result<(), String> {
    try_wait_state_until(output, screen, until, |screen| ready(&screen.contents())).await
}

async fn try_wait_state_until(
    output: &mut yolo_client::Attachment,
    screen: &mut vt100::Parser,
    until: tokio::time::Instant,
    ready: impl Fn(&vt100::Screen) -> bool,
) -> Result<(), String> {
    let result = timeout_at(until, async {
        loop {
            if tokio::time::Instant::now() >= until {
                return Err(format!(
                    "TUI screen did not settle:\n{}",
                    screen.screen().contents()
                ));
            }
            if ready(screen.screen()) {
                return Ok(());
            }
            let message = output.next_event().await.map_err(|error| {
                format!(
                    "TUI driver output ended: {error}; screen:\n{}",
                    screen.screen().contents()
                )
            });
            let message = message?;
            if message.event == event::TERMINAL_OUTPUT {
                let chunk: yolo_runtime::model::OutputChunk = from_value(message.payload).unwrap();
                screen.process(&chunk.data);
            }
        }
    })
    .await;
    result.unwrap_or_else(|_| {
        Err(format!(
            "TUI screen did not settle:\n{}",
            screen.screen().contents()
        ))
    })
}
fn quote(path: &Path) -> String {
    path.to_string_lossy().replace('\'', QUOTE_ESCAPE)
}

pub(super) fn launch(binary: &Path, node: &RunningNode, workspace: &Path) -> (String, String) {
    launch_at(binary, &node.socket, node.node.hub_id(), workspace)
}

pub(super) fn launch_at(
    binary: &Path,
    socket: &Path,
    hub: yolo_hub_identity::HubId,
    workspace: &Path,
) -> (String, String) {
    let nonce = uuid::Uuid::new_v4();
    let command = format!(
        "{CLIENT_STATE_ENV}='{}' '{}' {TERMINALS_COMMAND} --socket '{}' --hub {}; printf '%s%s\\n' '{COMPLETE_PREFIX}' '{nonce}'\n",
        quote(workspace),
        quote(binary),
        quote(socket),
        hub,
    );
    (command, format!("{COMPLETE_PREFIX}{nonce}"))
}

const QUOTE_ESCAPE: &str = "'\"'\"'";
const CLIENT_STATE_ENV: &str = "YOLO_TUI_STATE_DIRECTORY";
const COMPLETE_PREFIX: &str = "YOLO_TUI_RETURNED_";

pub(super) fn ping() -> (Vec<u8>, String) {
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let nonce = &nonce[..MARKER_NONCE_BYTES];
    let command = format!(
        "{} {MARKER_LEFT} {MARKER_RIGHT} {nonce}\r",
        crate::fake_agent_constants::PACKAGED_TUI_PING_COMMAND
    );
    (
        command.into_bytes(),
        format!("{MARKER_LEFT}{MARKER_RIGHT}{nonce}"),
    )
}

const MARKER_LEFT: &str = "T";
const MARKER_RIGHT: &str = "_";
const MARKER_NONCE_BYTES: usize = 8;
