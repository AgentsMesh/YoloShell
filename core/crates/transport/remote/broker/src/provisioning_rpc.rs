use super::{
    BrokerConnectionPool, BrokerResponse, BrokerResult, protocol::BrokerRequest,
    server_admission::BrokerAdmission,
};
use crate::local_unix::LocalUnixStream;
use anyhow::{Result, ensure};
use tokio::{
    io::AsyncReadExt,
    time::{Instant, timeout_at},
};
use uuid::Uuid;
use yolo_broker_hub_protocol::BrokerHubError;
use yolo_remote_provisioning_protocol::{BrokerProvisionCall, PROVISION_OPERATION_TIMEOUT};

const PROVISION_RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Clone, Copy)]
pub(super) enum Action {
    Provision,
    Recover,
    Advance,
}
pub(super) fn select(request: &BrokerRequest) -> Option<(&BrokerProvisionCall, Action)> {
    match request {
        BrokerRequest::ProvisionHub(call) => Some((call, Action::Provision)),
        BrokerRequest::RecoverHubProvision(call) => Some((call, Action::Recover)),
        BrokerRequest::AdvanceHubProvision(call) => Some((call, Action::Advance)),
        _ => None,
    }
}
pub(super) async fn serve(
    stream: &mut LocalUnixStream,
    pool: &BrokerConnectionPool,
    admission: &BrokerAdmission,
    request_id: Uuid,
    call: &BrokerProvisionCall,
    action: Action,
) -> Result<()> {
    let deadline = Instant::now() + PROVISION_OPERATION_TIMEOUT;
    let guard = || {
        ensure!(admission.is_adopted(), BrokerHubError::StaleBinding);
        Ok(())
    };
    let mut cancellation = [0];
    let operation = pool.provision_hub(call, action, deadline, &guard);
    let result = tokio::select! {
        biased;
        _ = stream.read(&mut cancellation) => return Ok(()),
        result = timeout_at(deadline, operation) => result.map_err(anyhow::Error::from).and_then(|value| value),
    };
    let response = match result {
        Ok(status) => BrokerResponse::success(request_id, BrokerResult::HubProvision(status)),
        Err(error) => super::hub_peer_response::failure(request_id, &error),
    };
    timeout_at(
        Instant::now() + PROVISION_RESPONSE_TIMEOUT,
        crate::protocol::write_frame(stream, &response),
    )
    .await??;
    Ok(())
}
