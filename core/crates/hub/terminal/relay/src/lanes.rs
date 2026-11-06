use crate::commands::SemanticOperation;
use tokio::task::{JoinError, JoinHandle};
use tokio::time::Instant;
use yolo_hub_client::{ClientError, LocalClient};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::TerminalSize;

pub(crate) type CallResult = (LocalClient, Result<Reply, ClientError>);
pub(crate) type InputTask = JoinHandle<CallResult>;
pub(crate) type RenewalTask = JoinHandle<(LocalClient, Result<Reply, ClientError>, Instant)>;
pub(crate) type ResizeTask =
    JoinHandle<(LocalClient, Result<Reply, ClientError>, bool, TerminalSize)>;
pub(crate) type SemanticTask =
    JoinHandle<(LocalClient, Result<Reply, ClientError>, SemanticOperation)>;

pub(crate) async fn wait<T>(task: &mut Option<JoinHandle<T>>) -> Option<Result<T, JoinError>> {
    match task.as_mut() {
        Some(task) => Some(task.await),
        None => std::future::pending().await,
    }
}
