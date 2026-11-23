use super::{
    NativeControlLookupRequest, NativeInterruptRequest, NativeObservationRequest,
    NativeRespondRequest, NativeSubmitRequest, constants::*,
};
use crate::protocol::{ErrorBody, RequestEnvelope, ResponseEnvelope, method};
use crate::worker::{AuthenticatedController, WorkerState};
use std::sync::Arc;

pub(in crate::worker) async fn dispatch(
    request: &RequestEnvelope,
    state: &Arc<WorkerState>,
    controller: AuthenticatedController,
) -> Result<ResponseEnvelope, ErrorBody> {
    match request.method.as_str() {
        method::OBSERVE_NATIVE_AGENT => {
            let payload: NativeObservationRequest = decode(request)?;
            let result = state.observe_native_agent(controller, &payload).await?;
            ResponseEnvelope::success(request, &result).map_err(super::super::protocol_error)
        }
        method::GET_NATIVE_CONTROL_RECEIPT => {
            let payload: NativeControlLookupRequest = decode(request)?;
            let result = state.native_control_receipt(controller, &payload).await?;
            ResponseEnvelope::success(request, &result).map_err(super::super::protocol_error)
        }
        method::INTERRUPT_NATIVE_AGENT => {
            let payload: NativeInterruptRequest = decode(request)?;
            let result = state.interrupt_native_agent(controller, &payload).await?;
            ResponseEnvelope::success(request, &result).map_err(super::super::protocol_error)
        }
        method::SUBMIT_NATIVE_AGENT => {
            let payload: NativeSubmitRequest = decode(request)?;
            let result = state.submit_native_agent(controller, &payload).await?;
            ResponseEnvelope::success(request, &result).map_err(super::super::protocol_error)
        }
        method::RESPOND_NATIVE_AGENT => {
            let payload: NativeRespondRequest = decode(request)?;
            let result = state.respond_native_agent(controller, &payload).await?;
            ResponseEnvelope::success(request, &result).map_err(super::super::protocol_error)
        }
        _ => Err(ErrorBody::invalid(INVALID_REQUEST)),
    }
}

fn decode<T: serde::de::DeserializeOwned>(request: &RequestEnvelope) -> Result<T, ErrorBody> {
    request
        .decode_payload()
        .map_err(|_| ErrorBody::invalid(INVALID_REQUEST))
}
