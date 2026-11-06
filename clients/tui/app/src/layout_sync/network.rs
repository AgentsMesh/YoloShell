use super::Operation;
use crate::{Options, constants::RPC_TIMEOUT, network::Update};
use tokio::{sync::mpsc, time::Instant};
use yolo_hub_rpc_protocol::{Command, Reply};

pub(crate) async fn call(options: &Options, command: Command) -> Result<Reply, String> {
    let deadline = Instant::now() + RPC_TIMEOUT;
    let mut client = options
        .endpoint
        .connect(options.hub, deadline)
        .await
        .map_err(|error| error.to_string())?;
    client
        .call(options.hub, command, deadline)
        .await
        .map_err(|error| error.to_string())
}

pub(crate) async fn run(
    options: &Options,
    mut operations: mpsc::Receiver<Operation>,
    updates: &mpsc::Sender<Update>,
) {
    while let Some(operation) = operations.recv().await {
        let result = call(options, operation.command.clone()).await;
        if updates
            .send(Update::Layout { operation, result })
            .await
            .is_err()
        {
            break;
        }
    }
}
