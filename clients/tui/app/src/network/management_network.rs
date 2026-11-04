use crate::{Options, network::Update};
use tokio::{sync::mpsc, time::Instant};
use yolo_tui_hub_manager::Operation;

pub const QUEUE_CAPACITY: usize = 1;
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub async fn run(
    options: &Options,
    mut requests: mpsc::Receiver<Operation>,
    updates: &mpsc::Sender<Update>,
) {
    while let Some(operation) = requests.recv().await {
        let result = yolo_hub_management_client::execute_endpoint(
            &options.endpoint,
            options.hub,
            operation.command.clone(),
            Instant::now() + REQUEST_TIMEOUT,
        )
        .await;
        let _ = updates.send(Update::Management { operation, result }).await;
    }
}
