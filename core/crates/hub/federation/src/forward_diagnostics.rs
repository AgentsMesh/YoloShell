use crate::Federation;
use uuid::Uuid;
use yolo_hub_identity::HubId;
use yolo_hub_peer_protocol::required_rights;
use yolo_hub_rpc_protocol::Request;
use yolo_hub_terminal_protocol::PublicError;

const FORWARD_FAILURE: &str = "Hub forwarding boundary failed";

#[derive(Clone, Copy, Debug)]
pub(crate) enum ForwardStage {
    EntryState,
    EntryRetired,
    RouteLookup,
    MissingRoute,
    BeforeForward,
    MissingNextHop,
    MissingLink,
    ChannelOpen,
    BeforeSend,
    Send,
    Receive,
    AfterReceive,
    InvalidResponse,
    OriginRejected,
    PeerRejected,
    InvalidRejection,
    PeerClosed,
    UnexpectedResponse,
    IngressRejected,
    OriginDispatch,
    OriginTimeout,
}

impl Federation {
    /// Log fixed identifiers and classifications only, never request/reply
    /// bodies, signed authority, transport error text or credential material.
    /// try_read cannot add a blocking wait to an already failing operation.
    pub(crate) fn forward_failure(
        &self,
        request: &Request,
        entry: HubId,
        next_hop: Option<HubId>,
        selected_generation: Option<Uuid>,
        stage: ForwardStage,
        code: PublicError,
    ) -> PublicError {
        let (sampled, current_generation, current_link_closed) = match self.connections.try_read() {
            Ok(current) => {
                let link = next_hop.and_then(|peer| current.get(&peer));
                (
                    true,
                    link.map(|link| link.id),
                    link.map(|link| link.connection.is_closed()),
                )
            }
            Err(_) => (false, None, None),
        };
        tracing::warn!(
            message = FORWARD_FAILURE,
            ?stage,
            entry_hub = %entry,
            hub = %self.hub_id(),
            destination = %request.fields().destination,
            request_id = %request.fields().request_id,
            method = ?request.fields().command.method(),
            required_rights = ?required_rights(&request.fields().command),
            ?next_hop,
            ?selected_generation,
            ?current_generation,
            ?current_link_closed,
            connection_state_sampled = sampled,
            policy_epoch = self.policy_epoch,
            closing = self.is_closing(),
            ?code,
        );
        code
    }
}
