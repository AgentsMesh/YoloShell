use super::{BrokerResponse, constants::*};
use crate::remote::RemoteError;
use uuid::Uuid;
use yolo_broker_hub_protocol::BrokerHubError;

pub(super) fn failure(request: Uuid, error: &anyhow::Error) -> BrokerResponse {
    let code = if let Some(error) = error.downcast_ref::<BrokerHubError>() {
        error.code()
    } else if error
        .downcast_ref::<RemoteError>()
        .is_some_and(RemoteError::requires_connection_recovery)
    {
        CONNECTION_LOST_ERROR_CODE
    } else {
        BROKER_ERROR_CODE
    };
    BrokerResponse::failure(request, code, error.to_string())
}
