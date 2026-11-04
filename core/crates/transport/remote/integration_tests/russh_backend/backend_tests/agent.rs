use super::constants::{
    AGENT_SOCKET, BROKEN_AGENT_SOCKET, CERTIFICATE_AGENT_SOCKET, SINGLE_AGENT_IDENTITY,
    SSH_AGENT_EMPTY_COMMENT, SSH_AGENT_NO_FLAGS, SSH_AGENT_SIGN_REQUEST_MESSAGE,
    SSH_AGENT_SIGN_RESPONSE_MESSAGE,
};
use crate::remote::constants::{
    MAX_SSH_AGENT_FRAME_BYTES, SSH_AGENT_IDENTITIES_ANSWER_MESSAGE, SSH_AGENT_LENGTH_PREFIX_BYTES,
    SSH_AGENT_REQUEST_IDENTITIES_MESSAGE,
};
use futures::Stream;
use russh::keys::agent::client::AgentClient;
use russh::keys::signature::Signer;
use russh::keys::ssh_encoding::{Decode, Encode};
use russh::keys::{Certificate, Error, PrivateKey};
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};

pub async fn start(directory: &std::path::Path, identity: &PrivateKey) -> PathBuf {
    start_many(directory, std::slice::from_ref(identity)).await
}

pub async fn start_many(directory: &std::path::Path, identities: &[PrivateKey]) -> PathBuf {
    let path = directory.join(AGENT_SOCKET);
    let listener = UnixListener::bind(&path).unwrap();
    tokio::spawn(async move {
        let _ = russh::keys::agent::server::serve(Incoming(listener), ()).await;
    });
    let mut client = AgentClient::connect_uds(&path).await.unwrap();
    for identity in identities {
        client.add_identity(identity, &[]).await.unwrap();
    }
    path
}

pub async fn start_broken(directory: &std::path::Path) -> PathBuf {
    let path = directory.join(BROKEN_AGENT_SOCKET);
    let listener = UnixListener::bind(&path).unwrap();
    tokio::spawn(async move {
        if let Ok((stream, _)) = listener.accept().await {
            drop(stream);
        }
    });
    path
}

pub async fn start_certificate(
    directory: &std::path::Path,
    identity: &PrivateKey,
    certificate: &Certificate,
) -> PathBuf {
    let path = directory.join(CERTIFICATE_AGENT_SOCKET);
    let listener = UnixListener::bind(&path).unwrap();
    let identity = identity.clone();
    let certificate = certificate.encode_vec().unwrap();
    tokio::spawn(async move {
        if let Ok((mut stream, _)) = listener.accept().await {
            let _ = serve_certificate(&mut stream, &identity, &certificate).await;
        }
    });
    path
}

async fn serve_certificate(
    stream: &mut UnixStream,
    identity: &PrivateKey,
    certificate: &[u8],
) -> Result<(), Error> {
    loop {
        let mut length = [0_u8; SSH_AGENT_LENGTH_PREFIX_BYTES];
        stream.read_exact(&mut length).await?;
        let length = u32::from_be_bytes(length) as usize;
        if length > MAX_SSH_AGENT_FRAME_BYTES {
            return Err(Error::AgentProtocolError);
        }
        let mut request = vec![0_u8; length];
        stream.read_exact(&mut request).await?;
        let response = certificate_response(&request, identity, certificate)?;
        let length = u32::try_from(response.len()).map_err(|_| Error::AgentProtocolError)?;
        stream.write_all(&length.to_be_bytes()).await?;
        stream.write_all(&response).await?;
        stream.flush().await?;
    }
}

fn certificate_response(
    request: &[u8],
    identity: &PrivateKey,
    certificate: &[u8],
) -> Result<Vec<u8>, Error> {
    let Some((&message, mut payload)) = request.split_first() else {
        return Err(Error::AgentProtocolError);
    };
    let mut response = Vec::new();
    match message {
        SSH_AGENT_REQUEST_IDENTITIES_MESSAGE if payload.is_empty() => {
            SSH_AGENT_IDENTITIES_ANSWER_MESSAGE.encode(&mut response)?;
            SINGLE_AGENT_IDENTITY.encode(&mut response)?;
            certificate.encode(&mut response)?;
            SSH_AGENT_EMPTY_COMMENT.encode(&mut response)?;
        }
        SSH_AGENT_SIGN_REQUEST_MESSAGE => {
            let requested = Vec::<u8>::decode(&mut payload)?;
            let data = Vec::<u8>::decode(&mut payload)?;
            let flags = u32::decode(&mut payload)?;
            if requested != certificate || flags != SSH_AGENT_NO_FLAGS || !payload.is_empty() {
                return Err(Error::AgentProtocolError);
            }
            let signature = identity
                .try_sign(&data)
                .map_err(|_| Error::AgentProtocolError)?;
            SSH_AGENT_SIGN_RESPONSE_MESSAGE.encode(&mut response)?;
            signature.encode_vec()?.encode(&mut response)?;
        }
        _ => return Err(Error::AgentProtocolError),
    }
    Ok(response)
}

struct Incoming(UnixListener);

impl Stream for Incoming {
    type Item = std::io::Result<UnixStream>;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0
            .poll_accept(context)
            .map(|result| Some(result.map(|(stream, _)| stream)))
    }
}
