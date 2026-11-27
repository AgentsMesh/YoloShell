use crate::{Options, run};
use yolo_hub_client::{ClientError, HubEndpoint};
use yolo_hub_identity::HubId;

#[tokio::test]
async fn invalid_entry_is_rejected_before_opening_terminal() {
    let result = run(Options {
        endpoint: HubEndpoint::local(""),
        hub: HubId::new(),
        layout: None,
    })
    .await;
    assert!(matches!(
        result.unwrap_err().downcast_ref::<ClientError>(),
        Some(ClientError::InvalidEndpoint)
    ));
}
