use super::*;
use yolo_runtime_protocol::{TerminalTextRequest, TerminalTextResult};

impl Client {
    pub async fn read_text(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Result<TerminalTextResult, ClientError> {
        self.read_text_until(terminal_id, runtime_epoch, self.operation_deadline())
            .await
    }

    pub(super) async fn read_text_until(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        deadline: ClientDeadline,
    ) -> Result<TerminalTextResult, ClientError> {
        self.request_until(
            method::GET_TEXT,
            &TerminalTextRequest {
                terminal_id,
                runtime_epoch,
            },
            deadline,
        )
        .await
    }
}
