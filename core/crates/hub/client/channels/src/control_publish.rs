use crate::{ControlError, ControlEvent, ControlRequest};
use tokio::sync::mpsc;
use yolo_hub_rpc_protocol::Reply;

const INPUT_BATCH_RESULT_MISSING: &str = "input batch result was consumed before publication";

pub(crate) async fn input_results(
    updates: &mpsc::Sender<ControlEvent>,
    originals: Vec<ControlRequest>,
    result: Result<Reply, ControlError>,
    dispatch: crate::input_completion::Dispatch,
    connected: bool,
) {
    let mut result = Some(result);
    for (index, request) in originals.into_iter().enumerate() {
        let current = if let Some(Ok(reply)) = result.as_ref() {
            Ok(reply.clone())
        } else if index == 0 {
            result.take().expect(INPUT_BATCH_RESULT_MISSING)
        } else {
            Err(ControlError::Invalidated)
        };
        crate::input_completion::settle(&request, &current, dispatch);
        let _ = updates
            .send(ControlEvent::Finished {
                request,
                result: current,
                connected,
            })
            .await;
    }
}
