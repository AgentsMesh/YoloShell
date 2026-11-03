use super::super::constants::*;
use super::{RemoteError, ssh_error};
use crate::local_unix::LocalUnixStream;
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::{AgentClient, AgentStream};
use russh::keys::ssh_encoding::Decode;
use std::io;
use std::path::Path;
use std::str;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

type DynamicAgentClient = AgentClient<Box<dyn AgentStream + Send + Unpin>>;

const SSH_AGENT_PEER_CREDENTIAL_CONTEXT: &str = "read SSH agent peer credentials";
const SSH_AGENT_PEER_UID_DETAIL: &str = "SSH agent peer effective uid does not match the client";

pub(super) async fn connect_env() -> Result<AgentClient<LocalUnixStream>, russh::keys::Error> {
    let path = match std::env::var(SSH_AUTH_SOCK_ENVIRONMENT) {
        Ok(path) => path,
        Err(_) => return Err(russh::keys::Error::EnvVar(SSH_AUTH_SOCK_ENVIRONMENT)),
    };
    match connect_uds(path).await {
        Err(russh::keys::Error::IO(error)) if error.kind() == io::ErrorKind::NotFound => {
            Err(russh::keys::Error::BadAuthSock)
        }
        result => result,
    }
}

pub(super) async fn connect_uds(
    path: impl AsRef<Path>,
) -> Result<AgentClient<LocalUnixStream>, russh::keys::Error> {
    let stream = LocalUnixStream::connect(path).await?;
    verify_peer_uid(&stream)?;
    Ok(AgentClient::connect(stream))
}

fn verify_peer_uid(stream: &LocalUnixStream) -> io::Result<()> {
    let peer = stream.peer_cred().map_err(|error| {
        io::Error::new(
            error.kind(),
            format!("{SSH_AGENT_PEER_CREDENTIAL_CONTEXT}: {error}"),
        )
    })?;
    // SAFETY: geteuid has no preconditions and only reads process credentials.
    validate_peer_uid(peer.uid(), unsafe { libc::geteuid() })
}

fn validate_peer_uid(peer: libc::uid_t, expected: libc::uid_t) -> io::Result<()> {
    if peer == expected {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        SSH_AGENT_PEER_UID_DETAIL,
    ))
}

pub(super) async fn request_bounded<S>(
    agent: AgentClient<S>,
) -> Result<(DynamicAgentClient, Vec<AgentIdentity>), RemoteError>
where
    S: AgentStream + Send + Unpin + 'static,
{
    let mut stream = agent.into_inner();
    let mut request = SSH_AGENT_SINGLE_MESSAGE_BYTES.to_be_bytes().to_vec();
    request.push(SSH_AGENT_REQUEST_IDENTITIES_MESSAGE);
    stream.write_all(&request).await.map_err(ssh_error)?;
    stream.flush().await.map_err(ssh_error)?;
    let mut length = [0_u8; SSH_AGENT_LENGTH_PREFIX_BYTES];
    stream.read_exact(&mut length).await.map_err(ssh_error)?;
    let length = u32::from_be_bytes(length) as usize;
    if length > MAX_SSH_AGENT_FRAME_BYTES {
        return Err(protocol_error());
    }
    let mut response = vec![0_u8; length];
    stream.read_exact(&mut response).await.map_err(ssh_error)?;
    let identities = decode_identities(&response)?;
    Ok((AgentClient::connect(stream), identities))
}

fn decode_identities(response: &[u8]) -> Result<Vec<AgentIdentity>, RemoteError> {
    let Some((&message, mut encoded)) = response.split_first() else {
        return Err(protocol_error());
    };
    if message != SSH_AGENT_IDENTITIES_ANSWER_MESSAGE {
        return Err(protocol_error());
    }
    let count = take_u32(&mut encoded)? as usize;
    if count > MAX_SSH_AGENT_IDENTITIES {
        return Err(RemoteError::transport(SSH_AGENT_IDENTITY_LIMIT_CONTEXT));
    }
    let mut identities = Vec::with_capacity(count);
    for _ in 0..count {
        let key_blob = take_string(&mut encoded)?;
        let comment = str::from_utf8(take_string(&mut encoded)?)
            .map_err(|_| protocol_error())?
            .to_owned();
        identities.push(decode_identity(key_blob, comment)?);
    }
    if !encoded.is_empty() {
        return Err(protocol_error());
    }
    Ok(identities)
}

fn decode_identity(key_blob: &[u8], comment: String) -> Result<AgentIdentity, RemoteError> {
    if is_certificate_blob(key_blob) {
        let mut encoded = key_blob;
        let certificate = russh::keys::Certificate::decode(&mut encoded).map_err(ssh_error)?;
        if !encoded.is_empty() {
            return Err(protocol_error());
        }
        return Ok(AgentIdentity::Certificate {
            certificate,
            comment,
        });
    }
    let key = russh::keys::PublicKey::from_bytes(key_blob).map_err(ssh_error)?;
    Ok(AgentIdentity::PublicKey { key, comment })
}

fn is_certificate_blob(blob: &[u8]) -> bool {
    let mut encoded = blob;
    take_string(&mut encoded)
        .ok()
        .and_then(|algorithm| str::from_utf8(algorithm).ok())
        .is_some_and(|algorithm| algorithm.ends_with(SSH_CERTIFICATE_ALGORITHM_SUFFIX))
}

fn take_u32(encoded: &mut &[u8]) -> Result<u32, RemoteError> {
    let Some((length, remaining)) = encoded.split_at_checked(SSH_AGENT_LENGTH_PREFIX_BYTES) else {
        return Err(protocol_error());
    };
    *encoded = remaining;
    Ok(u32::from_be_bytes(
        length.try_into().map_err(|_| protocol_error())?,
    ))
}

fn take_string<'a>(encoded: &mut &'a [u8]) -> Result<&'a [u8], RemoteError> {
    let length = take_u32(encoded)? as usize;
    let Some((value, remaining)) = encoded.split_at_checked(length) else {
        return Err(protocol_error());
    };
    *encoded = remaining;
    Ok(value)
}

fn protocol_error() -> RemoteError {
    RemoteError::transport(SSH_AGENT_PROTOCOL_MESSAGE)
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/russh_backend/agent_identities_tests.rs"]
mod tests;
