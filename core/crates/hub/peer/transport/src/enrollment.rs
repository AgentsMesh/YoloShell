use crate::{PeerStream, client, constants::*, pins::matches_key};
use russh::client as ssh_client;
use std::{fmt, num::NonZeroU16};
use tokio::net::TcpStream;
use tokio::time::{Instant, timeout_at};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_enrollment_protocol::SignedInvite;
use yolo_hub_enrollment_wire::{
    JoinErrorCode, JoinOutcome, JoinReply, JoinRequest, WireError, read_outcome, write_request,
};

/// Decides whether a pairing code authenticates an enrollment connection.
pub trait EnrollmentVerifier: Send + Sync + 'static {
    fn verify(&self, code: &str) -> Option<SignedInvite>;
}

/// One authenticated enrollment channel plus its verified invite.
pub struct EnrollmentStream {
    pub stream: PeerStream,
    pub invite: SignedInvite,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum JoinFailure {
    Connect,
    HostKeyMismatch,
    Authentication,
    Rejected(JoinErrorCode),
    Exchange,
}

impl fmt::Display for JoinFailure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => formatter.write_str("could not connect to the inviting Hub"),
            Self::HostKeyMismatch => {
                formatter.write_str("the address is not the inviting Hub (SSH host key mismatch)")
            }
            Self::Authentication => formatter.write_str("the inviting Hub rejected the pairing code"),
            Self::Rejected(code) => write!(formatter, "{code}"),
            Self::Exchange => formatter.write_str("the enrollment exchange failed"),
        }
    }
}

impl std::error::Error for JoinFailure {}

/// The result of accepting one inbound SSH connection.
pub enum Accepted {
    Peer {
        connection: crate::PeerConnection,
        incoming: tokio::sync::mpsc::Receiver<PeerStream>,
    },
    Enrollment {
        connection: crate::PeerConnection,
        incoming: tokio::sync::mpsc::Receiver<EnrollmentStream>,
    },
}

struct JoinClientHandler {
    expected: PeerIdentity,
}

impl ssh_client::Handler for JoinClientHandler {
    type Error = anyhow::Error;

    async fn check_server_key(
        &mut self,
        key: &russh::keys::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(matches_key(self.expected, key))
    }
}

/// Authenticates the invited Hub by host key, exchanges one join request and
/// closes the SSH session. The code is only sent after host-key verification.
pub async fn join(
    host: &str,
    port: NonZeroU16,
    code: &str,
    invite: &SignedInvite,
    request: &JoinRequest,
    deadline: Instant,
) -> Result<JoinReply, JoinFailure> {
    let stream = timeout_at(deadline, TcpStream::connect((host, port.get())))
        .await
        .map_err(|_| JoinFailure::Connect)?
        .map_err(|_| JoinFailure::Connect)?;
    let handler = JoinClientHandler {
        expected: invite.claims().issuer,
    };
    let mut handle = timeout_at(
        deadline,
        ssh_client::connect_stream(client::config(), stream, handler),
    )
    .await
    .map_err(|_| JoinFailure::Connect)?
    .map_err(|error| {
        if error
            .downcast_ref::<russh::Error>()
            .is_some_and(|error| matches!(error, russh::Error::UnknownKey))
        {
            JoinFailure::HostKeyMismatch
        } else {
            JoinFailure::Connect
        }
    })?;
    let authenticated = timeout_at(deadline, handle.authenticate_password(ENROLL_USER, code))
        .await
        .map_err(|_| JoinFailure::Exchange)?
        .map_err(|_| JoinFailure::Exchange)?;
    if !authenticated.success() {
        return Err(JoinFailure::Authentication);
    }
    let channel = timeout_at(deadline, handle.channel_open_session())
        .await
        .map_err(|_| JoinFailure::Exchange)?
        .map_err(|_| JoinFailure::Exchange)?;
    let mut stream = channel.into_stream();
    timeout_at(deadline, write_request(&mut stream, request))
        .await
        .map_err(|_| JoinFailure::Exchange)?
        .map_err(map_wire_error)?;
    let outcome = timeout_at(deadline, read_outcome(&mut stream))
        .await
        .map_err(|_| JoinFailure::Exchange)?
        .map_err(map_wire_error)?;
    // Closing the channel after a successful read lets the server confirm the
    // reply was consumed before it retires the SSH session.
    let _ = timeout_at(deadline, tokio::io::AsyncWriteExt::shutdown(&mut stream)).await;
    match outcome {
        JoinOutcome::Accepted { reply } if reply.hub == invite.claims().issuer => Ok(reply),
        JoinOutcome::Accepted { .. } => Err(JoinFailure::Exchange),
        JoinOutcome::Rejected { code } => Err(JoinFailure::Rejected(code)),
    }
}

fn map_wire_error(error: WireError) -> JoinFailure {
    match error {
        WireError::Rejected(code) => JoinFailure::Rejected(code),
        WireError::Io(_) | WireError::Codec | WireError::Size => JoinFailure::Exchange,
    }
}
