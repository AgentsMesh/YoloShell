use crate::LinkError;
use crate::frame;
use crate::message::{Hello, NONCE_BYTES, ROLE_CLIENT, ROLE_SERVER};
use crate::Admission;
use ring::rand::{SecureRandom, SystemRandom};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    time::Instant,
};
use yolo_hub_delegation::{PeerIdentity, Signer};

pub(crate) async fn client(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    local: PeerIdentity,
    signer: &impl Signer,
    expected_remote: PeerIdentity,
    deadline: Instant,
) -> Result<(), LinkError> {
    if signer.identity() != local {
        return Err(LinkError::Identity);
    }
    let nonce = random_nonce()?;
    frame::write(
        stream,
        &Hello::new(ROLE_CLIENT, local, nonce, None, signer)?,
        deadline,
    )
    .await?;
    let reply: Hello = frame::read(stream, deadline).await?;
    let remote = reply.verify_signature(ROLE_SERVER, Some(nonce))?;
    if remote != expected_remote {
        return Err(LinkError::Identity);
    }
    Ok(())
}

pub(crate) async fn server(
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    local: PeerIdentity,
    signer: &impl Signer,
    admission: Admission<'_>,
    deadline: Instant,
) -> Result<PeerIdentity, LinkError> {
    if signer.identity() != local {
        return Err(LinkError::Identity);
    }
    let hello: Hello = frame::read(stream, deadline).await?;
    let peer = hello.verify_signature(ROLE_CLIENT, None)?;
    if let Admission::Only(allowed_clients) = admission {
        if !allowed_clients.contains(&peer) {
            return Err(LinkError::Identity);
        }
    }
    let nonce = random_nonce()?;
    frame::write(
        stream,
        &Hello::new(ROLE_SERVER, local, nonce, Some(hello.nonce()?), signer)?,
        deadline,
    )
    .await?;
    Ok(peer)
}

fn random_nonce() -> Result<[u8; NONCE_BYTES], LinkError> {
    let mut nonce = [0; NONCE_BYTES];
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| LinkError::Signature)?;
    Ok(nonce)
}
