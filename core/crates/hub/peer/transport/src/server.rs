use crate::{EnrollmentStream, EnrollmentVerifier, PeerPins, PeerStream, constants::*};
use russh::{
    Channel,
    keys::{PrivateKey, PublicKey},
    server,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::{Semaphore, mpsc};
use yolo_hub_delegation::PeerIdentity;
use yolo_hub_enrollment_protocol::SignedInvite;

pub(crate) enum AuthEvent {
    Peer(PeerIdentity),
    Enrollment,
}

pub(crate) struct ServerHandler {
    pub pins: PeerPins,
    pub authenticated: mpsc::Sender<AuthEvent>,
    pub peer: Option<PeerIdentity>,
    pub invite: Option<SignedInvite>,
    pub verifier: Option<Arc<dyn EnrollmentVerifier>>,
    pub incoming: mpsc::Sender<PeerStream>,
    pub enrollment: mpsc::Sender<EnrollmentStream>,
    pub capacity: Arc<Semaphore>,
    pub closed: Arc<AtomicBool>,
}
impl Drop for ServerHandler {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Release);
    }
}
impl server::Handler for ServerHandler {
    type Error = anyhow::Error;
    async fn auth_publickey_offered(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        Ok(if user == PEER_USER && self.pins.identify(key).is_some() {
            server::Auth::Accept
        } else {
            server::Auth::reject()
        })
    }
    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        if user != PEER_USER || self.peer.is_some() || self.invite.is_some() {
            return Ok(server::Auth::reject());
        }
        let Some(peer) = self.pins.identify(key) else {
            return Ok(server::Auth::reject());
        };
        if self
            .authenticated
            .try_send(AuthEvent::Peer(peer))
            .is_err()
        {
            return Ok(server::Auth::reject());
        }
        self.peer = Some(peer);
        Ok(server::Auth::Accept)
    }
    async fn auth_password(
        &mut self,
        user: &str,
        password: &str,
    ) -> Result<server::Auth, Self::Error> {
        if user != ENROLL_USER || self.peer.is_some() || self.invite.is_some() {
            return Ok(server::Auth::reject());
        }
        let Some(verifier) = &self.verifier else {
            return Ok(server::Auth::reject());
        };
        let Some(invite) = verifier.verify(password) else {
            return Ok(server::Auth::reject());
        };
        if self
            .authenticated
            .try_send(AuthEvent::Enrollment)
            .is_err()
        {
            return Ok(server::Auth::reject());
        }
        self.invite = Some(invite);
        Ok(server::Auth::Accept)
    }
    async fn channel_open_session(
        &mut self,
        channel: Channel<server::Msg>,
        reply: server::ChannelOpenHandle,
        _: &mut server::Session,
    ) -> Result<(), Self::Error> {
        if let Some(invite) = &self.invite {
            let Ok(capacity) = self.capacity.clone().try_acquire_owned() else {
                return Ok(());
            };
            let Ok(queued) = self.enrollment.clone().try_reserve_owned() else {
                return Ok(());
            };
            reply.accept().await;
            queued.send(EnrollmentStream {
                stream: PeerStream::new(channel.into_stream(), capacity),
                invite: invite.clone(),
            });
            return Ok(());
        }
        if self.peer.is_none() {
            return Ok(());
        }
        let Ok(capacity) = self.capacity.clone().try_acquire_owned() else {
            return Ok(());
        };
        let Ok(queued) = self.incoming.clone().try_reserve_owned() else {
            return Ok(());
        };
        reply.accept().await;
        queued.send(PeerStream::new(channel.into_stream(), capacity));
        Ok(())
    }
}
pub(crate) fn config(key: Arc<PrivateKey>, enrollment: bool) -> Arc<server::Config> {
    let methods = if enrollment {
        russh::MethodSet::from(&[russh::MethodKind::PublicKey, russh::MethodKind::Password][..])
    } else {
        russh::MethodSet::from(&[russh::MethodKind::PublicKey][..])
    };
    Arc::new(server::Config {
        keys: vec![(*key).clone()],
        methods,
        channel_buffer_size: SSH_CHANNEL_MESSAGES,
        event_buffer_size: SSH_SESSION_MESSAGES,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: MISSED_KEEPALIVES,
        auth_rejection_time: AUTH_REJECTION_DELAY,
        ..Default::default()
    })
}
