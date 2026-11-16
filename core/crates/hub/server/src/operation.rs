use crate::constants::OPERATION_TIMEOUT;
use yolo_hub_dispatch::{Dispatcher, Operation};
use yolo_hub_rpc_protocol::Request;
use yolo_hub_terminal_protocol::PublicError;

pub(crate) async fn dispatch(
    dispatcher: &dyn Dispatcher,
    request: &Request,
) -> Result<Operation, PublicError> {
    let command = &request.fields().command;
    match tokio::time::timeout(
        command.operation_timeout(OPERATION_TIMEOUT),
        dispatcher.dispatch(request.clone()),
    )
    .await
    {
        Ok(result) => result,
        Err(_) => Err(if command.mutates() {
            PublicError::OutcomeUnknown
        } else {
            PublicError::Unreachable
        }),
    }
}
