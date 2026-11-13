use crate::{
    EntryProfile,
    connection::Connection,
    constants::*,
    error,
    handler::{self, Handler},
    socket::SocketOwner,
};
use russh::{
    client,
    keys::{PrivateKey, PrivateKeyWithHashAlg, PublicKey},
};
use std::{io::ErrorKind, sync::Arc};
use tokio::{
    net::TcpStream,
    sync::Mutex,
    time::{Instant, timeout_at},
};
use yolo_hub_client::ClientError;
use yolo_hub_client_transport::{ClientIo, connect as connect_inner};

pub(crate) async fn connect(
    profile: &EntryProfile,
    pin: PublicKey,
    outer_key: Arc<PrivateKey>,
    inner_key: Arc<PrivateKey>,
    lifecycle: &crate::lifecycle::Lifecycle,
    deadline: Instant,
) -> Result<Connection, ClientError> {
    let stream = timeout_at(
        deadline,
        TcpStream::connect((profile.host.as_str(), profile.port)),
    )
    .await
    .map_err(error::timeout)?
    .map_err(|_| error::transport(ErrorKind::ConnectionRefused, CONNECT_FAILED))?;
    let (stream, owner) = SocketOwner::capture(stream)
        .map_err(|_| error::transport(ErrorKind::ConnectionAborted, CONNECT_FAILED))?;
    if !lifecycle.retain(&owner.0) {
        return Err(error::transport(ErrorKind::NotConnected, ENTRY_CLOSED));
    }
    let handler = Handler {
        pin,
        socket: owner.0.clone(),
    };
    let mut outer = timeout_at(
        deadline,
        client::connect_stream(handler::config(), stream, handler),
    )
    .await
    .map_err(error::timeout)?
    .map_err(|_| error::transport(ErrorKind::PermissionDenied, CONNECT_FAILED))?;
    let hash = timeout_at(deadline, outer.best_supported_rsa_hash())
        .await
        .map_err(error::timeout)?
        .map_err(|_| error::transport(ErrorKind::PermissionDenied, AUTH_FAILED))?
        .flatten();
    let authentication = timeout_at(
        deadline,
        outer.authenticate_publickey(
            profile.username.clone(),
            PrivateKeyWithHashAlg::new(outer_key, hash),
        ),
    )
    .await
    .map_err(error::timeout)?
    .map_err(|_| error::transport(ErrorKind::PermissionDenied, AUTH_FAILED))?;
    if !authentication.success() {
        return Err(error::transport(ErrorKind::PermissionDenied, AUTH_FAILED));
    }
    let socket_path = profile
        .remote_socket
        .to_str()
        .ok_or(ClientError::InvalidEndpoint)?;
    let channel = timeout_at(deadline, outer.channel_open_direct_streamlocal(socket_path))
        .await
        .map_err(error::timeout)?
        .map_err(|_| error::transport(ErrorKind::PermissionDenied, INGRESS_FAILED))?;
    let close = owner.0.clone();
    let owned = ClientIo::new(channel.into_stream(), move || close.close());
    let inner = timeout_at(
        deadline,
        connect_inner(owned, inner_key, profile.hub, deadline),
    )
    .await
    .map_err(error::timeout)?
    .map_err(|_| error::stage(deadline, ErrorKind::PermissionDenied, HUB_AUTH_FAILED))?;
    Ok(Connection {
        inner,
        outer: Mutex::new(Some(outer)),
        owner,
    })
}
