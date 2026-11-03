use super::query_support::Fixture;
use crate::Mode;
use yolo_hub_client::ClientError;
use yolo_hub_client_channels::ControlError;
use yolo_hub_rpc_protocol::Reply;

pub(super) const PREFIX: &[u8] = b"printf '%s' 'original-prefix";
pub(super) const SUFFIX: &[u8] = b"'\r";

pub(super) fn navigate(fixture: &mut Fixture) {
    fixture.state.mode = Mode::Inventory;
    fixture.state.sync_query_authority();
}

pub(super) fn unknown() -> Result<Reply, ControlError> {
    Err(ControlError::Client(ClientError::Remote(
        yolo_hub_terminal_protocol::PublicError::OutcomeUnknown,
    )))
}
