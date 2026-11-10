use super::super::*;
use crate::remote_broker::provisioning_rpc::Action;
use std::os::unix::fs::PermissionsExt;
use tokio::net::UnixListener;
use yolo_remote_provisioning_protocol::{ProvisionOutcome, ProvisionPhase, ProvisionStatus};

const SOCKET_NAME: &str = "provision.sock";
const SOCKET_MODE: u32 = 0o600;
const TEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[tokio::test]
async fn typed_client_methods_preserve_binding_and_validate_full_result_request() {
    for action in [Action::Provision, Action::Recover, Action::Advance] {
        let root = tempfile::tempdir().unwrap();
        let socket = root.path().join(SOCKET_NAME);
        let listener = UnixListener::bind(&socket).unwrap();
        std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(SOCKET_MODE)).unwrap();
        let call = yolo_remote_provisioning_protocol_test_support::call();
        let expected = call.clone();
        let server = tokio::spawn(async move {
            let (mut connection, _) = listener.accept().await.unwrap();
            let request: BrokerRequestEnvelope =
                crate::protocol::read_frame(&mut connection).await.unwrap();
            let (actual, _) = provisioning_rpc::select(&request.request).unwrap();
            assert_eq!(actual, &expected);
            let status = ProvisionStatus::new(
                expected.request,
                ProvisionOutcome::Unknown {
                    phase: ProvisionPhase::Publishing,
                },
            )
            .unwrap();
            crate::protocol::write_frame(
                &mut connection,
                &BrokerResponse::success(request.request_id, BrokerResult::HubProvision(status)),
            )
            .await
            .unwrap();
        });
        let client = BrokerClient::new(socket);
        let deadline = tokio::time::Instant::now() + TEST_TIMEOUT;
        let status = match action {
            Action::Provision => client.provision_hub(call.clone(), deadline).await,
            Action::Recover => client.recover_hub_provision(call.clone(), deadline).await,
            Action::Advance => client.advance_hub_provision(call.clone(), deadline).await,
        }
        .unwrap();
        status.validate_for(&call.request).unwrap();
        server.await.unwrap();
    }
}
