use crate::{PeerDialFuture, PeerDialer, constants::UNSUPPORTED_TRANSPORT, require_budget};
use anyhow::ensure;
use tokio::{
    net::TcpStream,
    time::{Instant, timeout_at},
};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_federation_settings::{DialTransport, PeerAddress};
use yolo_hub_peer_transport::PeerIo;

#[derive(Debug, Default)]
pub struct TcpPeerDialer;

impl PeerDialer for TcpPeerDialer {
    fn dial<'a>(
        &'a self,
        _: PeerIdentity,
        address: &'a PeerAddress,
        deadline: Instant,
    ) -> PeerDialFuture<'a> {
        Box::pin(async move {
            require_budget(deadline)?;
            ensure!(
                matches!(&address.transport, DialTransport::Tcp),
                UNSUPPORTED_TRANSPORT
            );
            address.validate()?;
            let stream = timeout_at(
                deadline,
                TcpStream::connect((address.host.as_str(), address.port.get())),
            )
            .await??;
            require_budget(deadline)?;
            PeerIo::from_tcp(stream).map_err(Into::into)
        })
    }
}
