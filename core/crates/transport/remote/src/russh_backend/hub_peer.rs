use super::{
    RusshConnection, channel_transfer_custody::TransferredChannelCustody,
    hub_peer_stream::HubPeerStream,
};
use crate::remote::{ChannelPurpose, HubPeerRequest, OpenedHubPeerChannel, RemoteError};
use tokio::time::Instant;

pub(super) const HUB_PEER_ORIGINATOR_ADDRESS: &str = "127.0.0.1";
pub(super) const HUB_PEER_ORIGINATOR_PORT: u32 = 0;

pub(super) fn open_error(error: russh::Error) -> RemoteError {
    match error {
        russh::Error::ChannelOpenFailure(_) => RemoteError::ChannelRejected,
        error => super::ssh_error(error),
    }
}

impl RusshConnection {
    pub(super) async fn open_hub_peer_inner(
        &self,
        request: &HubPeerRequest,
    ) -> Result<OpenedHubPeerChannel, RemoteError> {
        if request.connection_id() != self.info.connection_id {
            return Err(RemoteError::ChannelMismatch);
        }
        let opened = self
            .open_hub_peer_channel_until(
                request.endpoint(),
                Instant::now() + self.operation_timeout,
            )
            .await?;
        let (session, channel, reservation) = opened.into_parts();
        let (stream, custody) = TransferredChannelCustody::start_hub_peer(
            session,
            channel,
            reservation,
            self.operation_timeout,
        );
        Ok(OpenedHubPeerChannel {
            identity: self.identity(ChannelPurpose::HubPeer),
            peer: request.peer(),
            stream: HubPeerStream::new(stream, custody),
        })
    }
}
