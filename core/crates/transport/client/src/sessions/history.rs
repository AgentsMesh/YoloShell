use super::*;
use yolo_runtime_protocol::{RetainedHistoryRequest, RetainedHistoryResult};

impl Client {
    pub async fn read_retained_history(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        max_bytes: u32,
    ) -> Result<RetainedHistoryResult, ClientError> {
        self.read_retained_history_until(
            terminal_id,
            runtime_epoch,
            max_bytes,
            self.operation_deadline(),
        )
        .await
    }

    pub(super) async fn read_retained_history_until(
        &self,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        max_bytes: u32,
        deadline: ClientDeadline,
    ) -> Result<RetainedHistoryResult, ClientError> {
        self.request_until(
            method::READ_RETAINED_HISTORY,
            &RetainedHistoryRequest {
                terminal_id,
                runtime_epoch,
                max_bytes,
            },
            deadline,
        )
        .await
    }
}
