//! Non-authoritative terminal text read: the Host validates the exact runtime
//! epoch and the Worker returns its visible screen.

use yolo_hub_local_errors::map_error;
use yolo_client::Client;
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{
    PublicError, ReadTerminalTextRequest, TerminalSize, TerminalText, TerminalTextFields,
};

pub async fn read_terminal_text(
    client: &Client,
    request: &ReadTerminalTextRequest,
) -> Result<Reply, PublicError> {
    let target = request.fields().target;
    let result = client
        .read_text(
            target.address.terminal_id.as_uuid(),
            target.runtime_epoch.as_uuid(),
        )
        .await
        .map_err(map_error)?;
    if result.runtime_epoch != target.runtime_epoch.as_uuid() {
        return Err(PublicError::OutcomeUnknown);
    }
    let text = TerminalText::new(TerminalTextFields {
        target,
        sequence: result.sequence,
        size: TerminalSize::new(result.cols, result.rows)?,
        text: result.text,
        truncated: result.truncated,
    })?;
    Ok(Reply::TerminalText { text })
}
