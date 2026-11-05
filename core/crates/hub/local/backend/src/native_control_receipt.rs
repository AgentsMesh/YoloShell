use crate::native_diagnostics::{self, RpcResult, RpcStage};
use yolo_agent_control_contract::NativeControlWitness;
use yolo_crypto::Sha256Digest;
use yolo_hub_agent_protocol::{AgentControlRequest, CommandOutcome, CommandReceipt, FailureCode};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::PublicError;
use yolo_runtime_protocol::NativeControlOutcome;

impl crate::LocalBackend {
    pub(crate) async fn resolve_native_receipt(
        &self,
        request: &AgentControlRequest,
        digest: Sha256Digest,
        witness: &NativeControlWitness,
        stage: RpcStage,
    ) -> Result<Option<Reply>, PublicError> {
        let fields = witness.fields();
        let result = self
            .client
            .native_control_receipt(
                yolo_runtime_protocol::RuntimeRef::new(fields.terminal_id, fields.runtime_epoch),
                request.fields().request_id,
                digest,
            )
            .await;
        let diagnostic = match &result {
            Ok(Some(outcome)) => RpcResult::from(*outcome),
            Ok(None) => RpcResult::Missing,
            Err(error) => RpcResult::client_error(error),
        };
        native_diagnostics::rpc_result(self.hub, request, stage, diagnostic);
        match result {
            Ok(Some(result)) => outcome(request, result).map(Some),
            Err(_) => outcome(request, NativeControlOutcome::Unknown).map(Some),
            Ok(None) => Ok(None),
        }
    }
}

const PUBLIC_COMMAND_DIGEST_DOMAIN: &str = "yolo.hub.native-control.public-request.v1";

pub(crate) fn command_digest(request: &AgentControlRequest) -> Result<Sha256Digest, PublicError> {
    Sha256Digest::capture(&(PUBLIC_COMMAND_DIGEST_DOMAIN, request))
        .map_err(|_| PublicError::Internal)
}
pub(crate) fn outcome(
    request: &AgentControlRequest,
    outcome: NativeControlOutcome,
) -> Result<Reply, PublicError> {
    let state = match outcome {
        NativeControlOutcome::Accepted => CommandOutcome::Accepted {},
        NativeControlOutcome::Confirmed => CommandOutcome::Confirmed {},
        NativeControlOutcome::Delivered => CommandOutcome::Delivered {},
        NativeControlOutcome::NotDelivered => CommandOutcome::Failed { code: FailureCode::AdapterRejected },
        NativeControlOutcome::Unknown => CommandOutcome::Unknown {},
    };
    let mut receipt =
        CommandReceipt::accepted(request.fields().request_id, request.fields().target)
            .map_err(|_| PublicError::Internal)?;
    receipt.advance(state).map_err(|_| PublicError::Internal)?;
    Ok(Reply::AgentControl { receipt })
}
pub(crate) fn reject(request: &AgentControlRequest, code: FailureCode) -> Reply {
    Reply::AgentControl {
        receipt: CommandReceipt::rejected(request, code),
    }
}
