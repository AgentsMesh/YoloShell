use crate::worker::{
    NativeControlDigest, NativeControlLookupRequest, NativeControlObservation,
    NativeControlOutcome, NativeControlResult, NativeInterruptRequest, NativeObservationRequest,
    NativeRespondRequest, NativeSubmitRequest, RuntimeRef,
};
use crate::{
    Client, ClientError,
    protocol::{ProtocolError, method},
};
use uuid::Uuid;

impl Client {
    pub async fn submit_native_agent(
        &self,
        request: &NativeSubmitRequest,
    ) -> Result<NativeControlResult, ClientError> {
        let result: NativeControlResult =
            self.request(method::SUBMIT_NATIVE_AGENT, request).await?;
        if !result.matches(
            request.terminal_id,
            request.runtime_epoch,
            request.request_id,
            request.command_digest,
        ) {
            return Err(ProtocolError::ResponseMismatch.into());
        }
        Ok(result)
    }

    pub async fn native_agent_observation(
        &self,
        target: RuntimeRef,
        binding_digest: NativeControlDigest,
    ) -> Result<Option<NativeControlObservation>, ClientError> {
        let result: Option<NativeControlObservation> = self
            .request(
                method::OBSERVE_NATIVE_AGENT,
                &NativeObservationRequest {
                    terminal_id: target.terminal_id,
                    runtime_epoch: target.runtime_epoch,
                    binding_digest,
                },
            )
            .await?;
        if let Some(observation) = &result {
            let witness = observation.witness().fields();
            if witness.terminal_id != target.terminal_id
                || witness.runtime_epoch != target.runtime_epoch
                || witness.binding_digest != binding_digest
            {
                return Err(ProtocolError::ResponseMismatch.into());
            }
        }
        Ok(result)
    }

    /// Resolves previously admitted effects without replay or writer-lease renewal.
    pub async fn native_control_receipt(
        &self,
        target: RuntimeRef,
        request_id: Uuid,
        command_digest: NativeControlDigest,
    ) -> Result<Option<NativeControlOutcome>, ClientError> {
        let result: Option<NativeControlResult> = self
            .request(
                method::GET_NATIVE_CONTROL_RECEIPT,
                &NativeControlLookupRequest {
                    terminal_id: target.terminal_id,
                    runtime_epoch: target.runtime_epoch,
                    request_id,
                    command_digest,
                },
            )
            .await?;
        if result.as_ref().is_some_and(|reply| {
            !reply.matches(
                target.terminal_id,
                target.runtime_epoch,
                request_id,
                command_digest,
            )
        }) {
            return Err(ProtocolError::ResponseMismatch.into());
        }
        Ok(result.map(|reply| reply.outcome))
    }

    pub async fn respond_native_agent(
        &self,
        request: &NativeRespondRequest,
    ) -> Result<NativeControlResult, ClientError> {
        let result: NativeControlResult = self
            .request(method::RESPOND_NATIVE_AGENT, request)
            .await?;
        if !result.matches(
            request.terminal_id,
            request.runtime_epoch,
            request.request_id,
            request.command_digest,
        ) {
            return Err(ProtocolError::ResponseMismatch.into());
        }
        Ok(result)
    }

    pub async fn interrupt_native_agent(
        &self,
        request: &NativeInterruptRequest,
    ) -> Result<NativeControlResult, ClientError> {
        let result: NativeControlResult = self
            .request(method::INTERRUPT_NATIVE_AGENT, request)
            .await?;
        if !result.matches(
            request.terminal_id,
            request.runtime_epoch,
            request.request_id,
            request.command_digest,
        ) {
            return Err(ProtocolError::ResponseMismatch.into());
        }
        Ok(result)
    }
}

#[cfg(all(test, feature = "yolo-unit-tests"))]
#[path = "../unit_tests/native_control_tests.rs"]
mod tests;
