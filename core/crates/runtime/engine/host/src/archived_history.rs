use super::*;
use yolo_runtime_protocol::{RetainedHistoryRequest, RetainedHistoryResult};

impl Host {
    pub(crate) async fn read_retained_history(
        &self,
        request: RetainedHistoryRequest,
    ) -> std::result::Result<RetainedHistoryResult, ErrorBody> {
        yolo_host_retained_history::read(self.namespace.as_ref(), &self.paths, request)
    }
}
