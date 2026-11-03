use crate::{
    LocalBackend,
    agent_control::reject_observation_only_control,
    native_control_receipt as receipt,
    native_diagnostics::{self, ObservationContext, Rejection, RpcStage},
};
use yolo_hub_agent_projection::AgentProjection;
use yolo_hub_agent_protocol::{AgentControlRequest, FailureCode, ProtocolError};
use yolo_hub_rpc_protocol::Reply;
use yolo_hub_terminal_protocol::{LeaseGrant, PublicError};

impl LocalBackend {
    pub(crate) async fn native_agent_control(
        &self,
        request: &AgentControlRequest,
        grant: LeaseGrant,
        projection: &mut AgentProjection,
    ) -> Result<Reply, PublicError> {
        let digest = receipt::command_digest(request)?;
        let retained = projection
            .retained_control_witness(request.fields().target)
            .map_err(|_| PublicError::Internal)?;
        if let Some(retained) = &retained {
            if let Some(reply) = self
                .resolve_native_receipt(request, digest, retained, RpcStage::ReceiptBeforeRefresh)
                .await?
            {
                return Ok(reply);
            }
        }
        let snapshots = self
            .observe_agents_locked(
                Some(&mut *projection),
                Some(ObservationContext {
                    request,
                    terminal: grant.fields().target,
                }),
                Some(grant.fields().target.address.terminal_id),
            )
            .await;
        // A lost call may arrive while refresh is awaited; inspect the receipt again.
        if let Some(retained) = &retained {
            if let Some(reply) = self
                .resolve_native_receipt(request, digest, retained, RpcStage::ReceiptAfterRefresh)
                .await?
            {
                return Ok(reply);
            }
        }
        let snapshots = snapshots?;
        let snapshot = snapshots
            .into_iter()
            .find(|snapshot| snapshot.cursor().target() == request.fields().target)
            .ok_or(PublicError::NotFound)?;
        if retained.is_none() {
            native_diagnostics::rejected(
                self.hub,
                request,
                &snapshot,
                grant.fields().target,
                Rejection::ObservationOnly,
            );
            // A binding alone cannot establish either past support or effect authority.
            return Ok(Reply::AgentControl {
                receipt: reject_observation_only_control(request, grant, &snapshot),
            });
        }
        if grant.fields().target != snapshot.fields().terminal {
            native_diagnostics::rejected(
                self.hub,
                request,
                &snapshot,
                grant.fields().target,
                Rejection::TerminalMismatch,
            );
            return Ok(receipt::reject(request, FailureCode::PreconditionsChanged));
        }
        if let Err(error) = request.validate_against(&snapshot) {
            native_diagnostics::rejected(
                self.hub,
                request,
                &snapshot,
                grant.fields().target,
                Rejection::Request(error),
            );
            return Ok(receipt::reject(
                request,
                if error == ProtocolError::UnsupportedControl {
                    FailureCode::Unsupported
                } else {
                    FailureCode::PreconditionsChanged
                },
            ));
        }
        let Some(witness) = projection
            .control_witness(request)
            .map_err(|_| PublicError::Internal)?
        else {
            native_diagnostics::rejected(
                self.hub,
                request,
                &snapshot,
                grant.fields().target,
                Rejection::WitnessUnavailable,
            );
            return Ok(receipt::reject(request, FailureCode::Unsupported));
        };
        self.execute_native_control(request, grant, witness, digest)
            .await
    }
}
