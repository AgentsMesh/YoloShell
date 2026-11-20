use crate::native_diagnostics::{self, RpcResult, RpcStage};
use crate::{LocalBackend, native_control_receipt as receipt, writer_conversion::internal_grant};
use yolo_agent_control_contract::NativeControlWitness;
use yolo_crypto::Sha256Digest;
use yolo_hub_agent_protocol::{
    AgentAction, AgentControlRequest, InteractionResponse,
};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError};
use yolo_runtime_protocol::{
    NativeControlOutcome, NativeInterruptRequest, NativeRespondRequest, NativeSubmitRequest,
};

impl LocalBackend {
    pub(crate) async fn execute_native_control(
        &self,
        request: &AgentControlRequest,
        grant: LeaseGrant,
        witness: NativeControlWitness,
        digest: Sha256Digest,
    ) -> Result<Reply, PublicError> {
        let authority = internal_grant(grant)?;
        let fallback_witness = witness.clone();
        let fields = witness.fields();
        let terminal_id = fields.terminal_id;
        let runtime_epoch = fields.runtime_epoch;
        let request_id = request.fields().request_id;
        let (result, stage) = match &request.fields().action {
            AgentAction::Interrupt {} => {
                let operation = NativeInterruptRequest {
                    terminal_id,
                    runtime_epoch,
                    request_id,
                    generation: authority.generation,
                    writer_capability: authority.writer_capability,
                    witness,
                    command_digest: digest,
                };
                (
                    self.client.interrupt_native_agent(&operation).await,
                    RpcStage::Interrupt,
                )
            }
            AgentAction::SubmitPrompt { prompt } => {
                let operation = NativeSubmitRequest {
                    terminal_id,
                    runtime_epoch,
                    request_id,
                    generation: authority.generation,
                    writer_capability: authority.writer_capability,
                    witness,
                    command_digest: digest,
                    prompt: prompt.clone(),
                };
                (
                    self.client.submit_native_agent(&operation).await,
                    RpcStage::Submit,
                )
            }
            AgentAction::Respond {
                interaction,
                response,
            } => {
                let InteractionResponse::Choice { choice } = response else {
                    return Err(PublicError::Unsupported);
                };
                let operation = NativeRespondRequest {
                    terminal_id,
                    runtime_epoch,
                    request_id,
                    generation: authority.generation,
                    writer_capability: authority.writer_capability,
                    witness,
                    command_digest: digest,
                    interaction: interaction.id,
                    choice: choice.clone(),
                };
                (
                    self.client.respond_native_agent(&operation).await,
                    RpcStage::Respond,
                )
            }
        };
        if result.is_err()
            && let Some(reply) = self
                .resolve_native_receipt(request, digest, &fallback_witness, RpcStage::ReceiptAfterRefresh)
                .await?
        {
            return Ok(reply);
        }
        let correlated = result
            .as_ref()
            .is_ok_and(|reply| reply.matches(terminal_id, runtime_epoch, request_id, digest));
        let diagnostic = match &result {
            Ok(reply) if correlated => RpcResult::from(reply.outcome),
            Ok(_) => RpcResult::CorrelationMismatch,
            Err(error) => RpcResult::client_error(error),
        };
        native_diagnostics::rpc_result(self.hub, request, stage, diagnostic);
        let outcome = match result {
            Ok(reply) if correlated => reply.outcome,
            _ => NativeControlOutcome::Unknown,
        };
        receipt::outcome(request, outcome)
    }
}
