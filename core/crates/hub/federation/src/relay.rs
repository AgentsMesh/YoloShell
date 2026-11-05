use crate::forward_diagnostics::ForwardStage;
use crate::{Federation, constants::*, node::now_ms};
use tokio::time::Instant;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_peer_protocol::{
    Call, ExecutionReply, PeerReply, PeerRequest, Signed, required_rights,
};
use yolo_hub_peer_rpc::RemoteUpdates;
use yolo_hub_terminal_protocol::PublicError;

impl Federation {
    pub(crate) async fn admit(
        &self,
        peer: PeerIdentity,
        call: &Signed<Call>,
    ) -> Result<(), PublicError> {
        self.require_open().map_err(|_| PublicError::Unreachable)?;
        let origin = call.payload.authority.origin();
        let (pin, epoch) = if origin.hub() == self.hub_id() {
            (self.state.peer_identity(), self.policy_epoch)
        } else {
            let book = self.book.lock().await;
            let pin = book.pin(origin.hub()).ok_or(PublicError::Unauthorized)?;
            let epoch = book
                .latest_for(origin.hub())
                .map(|source| source.payload.policy_epoch)
                .unwrap_or(call.payload.authority.policy_epoch());
            (pin, epoch)
        };
        let policy = self
            .config
            .export(peer.hub(), origin.hub())
            .ok_or(PublicError::Unauthorized)?;
        let Some(required) = required_rights(&call.payload.request.fields().command) else {
            // Local management never crosses a peer boundary.
            return Err(PublicError::Unauthorized);
        };
        if !policy.rights.allows(&required) {
            return Err(PublicError::Unauthorized);
        }
        self.require_open().map_err(|_| PublicError::Unreachable)?;
        call.admit(
            self.hub_id(),
            peer,
            pin,
            epoch,
            now_ms().map_err(|_| PublicError::Internal)?,
        )
        .map_err(|_| PublicError::Unauthorized)
    }

    pub(crate) async fn forward(
        &self,
        call: &Signed<Call>,
    ) -> Result<(Signed<ExecutionReply>, Option<RemoteUpdates>), PublicError> {
        let request = &call.payload.request;
        let entry = call.signer.hub();
        let before = |stage| {
            self.forward_failure(request, entry, None, None, stage, PublicError::Unreachable)
        };
        self.require_open()
            .map_err(|_| before(ForwardStage::BeforeForward))?;
        let next = call
            .next_hop(self.hub_id())
            .ok_or_else(|| before(ForwardStage::MissingNextHop))?;
        let connection = self.current(next).await.ok_or_else(|| {
            self.forward_failure(
                request,
                entry,
                Some(next),
                None,
                ForwardStage::MissingLink,
                PublicError::Unreachable,
            )
        })?;
        let failed = |stage, code| {
            self.forward_failure(request, entry, Some(next), Some(connection.id), stage, code)
        };
        let deadline = Instant::now()
            + request
                .fields()
                .command
                .operation_timeout(OPERATION_TIMEOUT);
        self.require_open()
            .map_err(|_| failed(ForwardStage::BeforeSend, PublicError::Unreachable))?;
        let mutation = call.payload.request.fields().command.mutates();
        let uncertain = || {
            if mutation {
                PublicError::OutcomeUnknown
            } else {
                PublicError::Unreachable
            }
        };
        let response = connection
            .client
            .request(PeerRequest::Execute { call: call.clone() }, deadline)
            .await
            .map_err(|_| failed(ForwardStage::Receive, uncertain()))?;
        self.require_open()
            .map_err(|_| failed(ForwardStage::AfterReceive, uncertain()))?;
        match response.reply {
            PeerReply::Executed { response: signed } => {
                signed
                    .validate_for(call)
                    .map_err(|_| failed(ForwardStage::InvalidResponse, uncertain()))?;
                if let yolo_hub_rpc_protocol::Reply::Error { code } = &signed.payload.response.reply
                {
                    failed(ForwardStage::OriginRejected, *code);
                }
                let attached = matches!(
                    signed.payload.response.reply,
                    yolo_hub_rpc_protocol::Reply::Attached { .. }
                );
                if attached != response.updates.is_some() {
                    return Err(failed(ForwardStage::InvalidResponse, uncertain()));
                }
                Ok((signed, response.updates))
            }
            PeerReply::Rejected {
                call_fingerprint,
                code,
            } => {
                if !call
                    .fingerprint()
                    .is_ok_and(|fingerprint| fingerprint == call_fingerprint)
                {
                    return Err(failed(ForwardStage::InvalidRejection, uncertain()));
                }
                failed(ForwardStage::PeerRejected, code);
                if matches!(
                    code,
                    PublicError::Unauthorized
                        | PublicError::Unreachable
                        | PublicError::OutcomeUnknown
                ) {
                    Err(code)
                } else {
                    Err(uncertain())
                }
            }
            _ => Err(failed(ForwardStage::UnexpectedResponse, uncertain())),
        }
    }
}
