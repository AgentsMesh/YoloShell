use anyhow::Result;
use std::sync::Arc;
use tokio::io::AsyncReadExt;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Instant, timeout};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_enrollment_protocol::{SignedInvite, decode};
use yolo_hub_enrollment_wire::{
    JoinErrorCode, JoinOutcome, JoinReply, JoinRequest, read_request, write_outcome,
};
use yolo_hub_peer_transport::{
    EnrollmentStream, EnrollmentVerifier, PeerConnection,
};

/// A verified invite and joining identity waiting for the configuration actor.
pub struct EnrollmentAdmission {
    pub invite: SignedInvite,
    pub request: JoinRequest,
    pub reply: oneshot::Sender<Result<JoinReply, JoinErrorCode>>,
}

pub type EnrollmentSender = mpsc::Sender<EnrollmentAdmission>;

#[derive(Clone)]
pub struct EnrollmentService {
    sender: EnrollmentSender,
    local: PeerIdentity,
}

struct InviteVerifier {
    local: PeerIdentity,
}

impl EnrollmentVerifier for InviteVerifier {
    fn verify(&self, code: &str) -> Option<SignedInvite> {
        let invite = decode(code).ok()?;
        let now = now_ms().ok()?;
        invite.verify_at(now).ok()?;
        (invite.claims().issuer == self.local).then_some(invite)
    }
}

impl EnrollmentService {
    pub fn new(sender: EnrollmentSender, local: PeerIdentity) -> Self {
        Self { sender, local }
    }

    pub fn verifier(&self) -> Arc<dyn EnrollmentVerifier> {
        Arc::new(InviteVerifier { local: self.local })
    }

    /// Spawns one bounded exchange. It captures no Hub generation: admitting
    /// the invite retires the generation whose listener accepted it before the
    /// reply is written.
    pub fn spawn(
        &self,
        connection: PeerConnection,
        incoming: mpsc::Receiver<EnrollmentStream>,
        permit: tokio::sync::OwnedSemaphorePermit,
    ) {
        let sender = self.sender.clone();
        tokio::spawn(async move {
            let _ = exchange(connection, incoming, sender, permit).await;
        });
    }
}

async fn exchange(
    connection: PeerConnection,
    mut incoming: mpsc::Receiver<EnrollmentStream>,
    sender: EnrollmentSender,
    _permit: tokio::sync::OwnedSemaphorePermit,
) -> Result<()> {
    let Some(EnrollmentStream { mut stream, invite }) =
        timeout(CHANNEL_TIMEOUT, incoming.recv()).await?
    else {
        let _ = connection.close(Instant::now() + CLOSE_TIMEOUT).await;
        return Ok(());
    };
    let request = match timeout(CHANNEL_TIMEOUT, read_request(&mut stream)).await {
        Ok(Ok(request)) => request,
        _ => {
            let _ = connection.close(Instant::now() + CLOSE_TIMEOUT).await;
            return Ok(());
        }
    };
    let outcome = if request.invite_id != invite.claims().invite_id {
        JoinOutcome::Rejected {
            code: JoinErrorCode::Unauthorized,
        }
    } else {
        admit(sender, invite, request).await
    };
    let _ = timeout(CHANNEL_TIMEOUT, write_outcome(&mut stream, &outcome)).await;
    // The client closes the channel after consuming the reply; waiting for
    // that EOF keeps the SSH session from retiring before delivery.
    let _ = timeout(CHANNEL_TIMEOUT, stream.read_u8()).await;
    let _ = connection.close(Instant::now() + CLOSE_TIMEOUT).await;
    Ok(())
}

async fn admit(
    sender: EnrollmentSender,
    invite: SignedInvite,
    request: JoinRequest,
) -> JoinOutcome {
    let (reply, result) = oneshot::channel();
    if sender
        .try_send(EnrollmentAdmission {
            invite,
            request,
            reply,
        })
        .is_err()
    {
        return JoinOutcome::Rejected {
            code: JoinErrorCode::Unavailable,
        };
    }
    match timeout(REPLY_TIMEOUT, result).await {
        Ok(Ok(Ok(reply))) => JoinOutcome::Accepted { reply },
        Ok(Ok(Err(code))) => JoinOutcome::Rejected { code },
        _ => JoinOutcome::Rejected {
            code: JoinErrorCode::Unavailable,
        },
    }
}

fn now_ms() -> Result<u64> {
    Ok(u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis(),
    )?)
}

const CHANNEL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);
const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
const CLOSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
