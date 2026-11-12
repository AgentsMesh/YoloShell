use super::constants::*;
use crate::hub::{multipath::io, relay_crash::process::RelayProcess};
use tokio::io::{AsyncRead, AsyncWrite};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_terminal_protocol::OutputCursor;
use yolo_hub_terminal_client::Attachment;

pub async fn exchange(
    entry: &RelayProcess,
    target: RuntimeTarget,
    output: &mut Attachment<impl AsyncRead + AsyncWrite + Unpin>,
) -> OutputCursor {
    let nonce = uuid::Uuid::new_v4();
    let command = format!("{SHELL_OUTPUT_COMMAND} '{nonce}'\n");
    let marker = format!("{SHELL_OUTPUT_PREFIX}{nonce}");
    io::exchange_command_client(
        entry.client().await,
        target,
        output,
        command.as_bytes(),
        &marker,
    )
    .await
}
