use crate::forward_diagnostics::ForwardStage;
use crate::lifetime::OwnedGeneration;
use crate::{Federation, constants::*};
use anyhow::Result;
use std::sync::Arc;
use tokio::time::timeout;
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_dispatch::Operation;
use yolo_hub_peer_protocol::{Call, ExecutionReply, PeerReply, PeerRequest, Purpose, Signed};
use yolo_hub_peer_io::Duplex;
use yolo_hub_peer_rpc::{Handler, Response as RpcResponse, RpcError, RpcFuture};
use yolo_hub_rpc_protocol::{Reply, Response};
use yolo_hub_terminal_protocol::PublicError;

impl Federation {
    pub(crate) async fn serve_peer(
        self: Arc<Self>,
        peer: PeerIdentity,
        stream: impl Duplex + 'static,
    ) -> Result<()> {
        self.require_open()?;
        let handler = PeerHandler {
            peer,
            generation: OwnedGeneration::new(self)?,
        };
        yolo_hub_peer_rpc::serve(stream, Arc::new(handler)).await?;
        Ok(())
    }
    async fn execute(
        self: Arc<Self>,
        peer: PeerIdentity,
        call: Signed<Call>,
    ) -> Result<RpcResponse> {
        if let Err(code) = self.admit(peer, &call).await {
            self.forward_failure(
                &call.payload.request,
                call.signer.hub(),
                call.next_hop(self.hub_id()),
                None,
                ForwardStage::IngressRejected,
                code,
            );
            return reject(&call, code);
        }
        if call.payload.request.fields().destination == self.hub_id() {
            return self.origin(peer, call).await;
        }
        match self.forward(&call).await {
            Ok((response, upstream)) => {
                self.require_open()?;
                if let Err(code) = self.admit_read_reply(peer, &call).await {
                    return reject(&call, code);
                }
                let updates = if matches!(response.payload.response.reply, Reply::Attached { .. }) {
                    Some(self.bridge(
                        call,
                        upstream.ok_or_else(|| anyhow::anyhow!(INVALID_DISCOVERY))?,
                    )?)
                } else {
                    None
                };
                Ok(RpcResponse {
                    reply: PeerReply::Executed { response },
                    updates,
                })
            }
            Err(code) => reject(&call, code),
        }
    }
    async fn origin(
        self: Arc<Self>,
        peer: PeerIdentity,
        call: Signed<Call>,
    ) -> Result<RpcResponse> {
        self.require_open()?;
        let result = timeout(
            call.payload
                .request
                .fields()
                .command
                .operation_timeout(OPERATION_TIMEOUT),
            self.origin.dispatch(call.payload.request.clone()),
        )
        .await;
        self.require_open()?;
        if let Err(code) = self.admit_read_reply(peer, &call).await {
            return reject(&call, code);
        }
        let (reply, source) = match result {
            Ok(Ok(Operation::Reply(reply))) => (reply, None),
            Ok(Ok(Operation::Attached(source))) => (
                Reply::Attached {
                    attachment: source.initial().clone(),
                },
                Some(source),
            ),
            Ok(Err(code)) => {
                self.forward_failure(
                    &call.payload.request,
                    call.signer.hub(),
                    None,
                    None,
                    ForwardStage::OriginDispatch,
                    code,
                );
                (Reply::Error { code }, None)
            }
            Err(_) => (
                Reply::Error {
                    code: self.forward_failure(
                        &call.payload.request,
                        call.signer.hub(),
                        None,
                        None,
                        ForwardStage::OriginTimeout,
                        if call.payload.request.fields().command.mutates() {
                            PublicError::OutcomeUnknown
                        } else {
                            PublicError::Unreachable
                        },
                    ),
                },
                None,
            ),
        };
        let response = Response::new(&call.payload.request, self.hub_id(), reply);
        response.validate_for(&call.payload.request)?;
        let response = Signed::new(
            ExecutionReply {
                call_fingerprint: call.fingerprint()?,
                response,
            },
            Purpose::Response,
            self.state.as_ref(),
        )?;
        let updates = source
            .map(|source| self.origin_stream(call, source))
            .transpose()?;
        Ok(RpcResponse {
            reply: PeerReply::Executed { response },
            updates,
        })
    }
}
struct PeerHandler {
    peer: PeerIdentity,
    generation: OwnedGeneration,
}
impl Handler for PeerHandler {
    fn request(&self, request: PeerRequest) -> RpcFuture<'_, RpcResponse> {
        Box::pin(async move {
            let node = self.generation.node();
            node.require_open().map_err(error)?;
            match request {
                PeerRequest::Discover {
                    request_id,
                    version,
                    allowed_origins,
                } => {
                    let reply = node
                        .directory(self.peer, request_id, version, allowed_origins)
                        .await
                        .map_err(error)?;
                    node.require_open().map_err(error)?;
                    Ok(RpcResponse {
                        reply,
                        updates: None,
                    })
                }
                PeerRequest::Execute { call } => {
                    node.clone().execute(self.peer, call).await.map_err(error)
                }
            }
        })
    }
}
fn reject(call: &Signed<Call>, code: PublicError) -> Result<RpcResponse> {
    Ok(RpcResponse {
        reply: PeerReply::Rejected {
            call_fingerprint: call.fingerprint()?,
            code,
        },
        updates: None,
    })
}
fn error(_: anyhow::Error) -> RpcError {
    RpcError::Invalid
}

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/lifetime_peer.rs"]
mod lifetime_integration;
