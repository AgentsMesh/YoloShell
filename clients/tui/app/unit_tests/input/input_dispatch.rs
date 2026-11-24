use crate::{input_dispatch, network::Update};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_identity::HubId;
use yolo_hub_rpc_protocol::Command;

const PRIORITY_CAPACITY: usize = 1;

#[tokio::test]
async fn not_sent_reports_wait_for_capacity_instead_of_dropping() {
    let (sender, mut receiver) = mpsc::channel(PRIORITY_CAPACITY);
    sender
        .send(Update::ManagementNotSent {
            token: Uuid::new_v4(),
        })
        .await
        .unwrap();
    let request = ControlRequest {
        generation: None,
        destination: HubId::new(),
        command: Command::Hello {},
        validity: None,
    };
    let expected_destination = request.destination;
    let expected_command = request.command.clone();
    let sender_clone = sender.clone();
    let pending = tokio::spawn(async move {
        input_dispatch::report_not_sent(&sender_clone, request).await;
    });
    tokio::task::yield_now().await;
    assert!(!pending.is_finished());

    assert!(matches!(
        receiver.recv().await,
        Some(Update::ManagementNotSent { .. })
    ));
    pending.await.unwrap();
    let Some(Update::NotSent { request: actual }) = receiver.recv().await else {
        panic!()
    };
    assert_eq!(actual.destination, expected_destination);
    assert_eq!(actual.command, expected_command);
}
