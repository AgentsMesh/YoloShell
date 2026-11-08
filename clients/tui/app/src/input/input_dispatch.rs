use crate::network::Update;
use tokio::sync::mpsc;
use yolo_hub_client_channels::ControlRequest;

pub(crate) async fn report_not_sent(
    priority_updates: &mpsc::Sender<Update>,
    request: ControlRequest,
) {
    let _ = priority_updates.send(Update::NotSent { request }).await;
}

pub(crate) fn enqueue(
    commands: &mpsc::Sender<ControlRequest>,
    request: ControlRequest,
) -> Option<ControlRequest> {
    commands
        .try_send(request)
        .err()
        .map(|error| error.into_inner())
}

/// A bounded first-delivery owner. A slow input transport cannot stop the
/// network router from forwarding lease renewals on their independent lane.
pub(crate) async fn run(
    mut incoming: mpsc::Receiver<ControlRequest>,
    commands: mpsc::Sender<ControlRequest>,
    priority_updates: mpsc::Sender<Update>,
) {
    while let Some(request) = incoming.recv().await {
        if let Err(error) = commands.send(request).await {
            report_not_sent(&priority_updates, error.0).await;
        }
    }
}
