use crate::{ClientStream, IncomingChannel, constants::*, custody::CloseState, time::now_ms};
use russh::{
    Channel,
    keys::{PrivateKey, PublicKey},
    server,
};
use std::sync::Arc;
use tokio::sync::{Semaphore, mpsc};
use uuid::Uuid;
use yolo_hub_client_access_registry::{AccessRegistry, ClientAdmission};

pub(crate) struct ServerHandler {
    pub registry: Option<AccessRegistry>,
    pub authenticated: mpsc::Sender<ClientAdmission>,
    pub admission: Option<ClientAdmission>,
    pub connection_id: Uuid,
    pub incoming: mpsc::Sender<IncomingChannel>,
    pub capacity: Arc<Semaphore>,
    pub transport: Arc<CloseState>,
}
impl Drop for ServerHandler {
    fn drop(&mut self) {
        self.transport.close();
    }
}
impl server::Handler for ServerHandler {
    type Error = anyhow::Error;
    async fn auth_publickey_offered(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        // A probe does not prove possession and must never publish admission.
        Ok(
            if user == PUBLIC_CLIENT_USER && key.key_data().ed25519().is_some() {
                server::Auth::Accept
            } else {
                server::Auth::reject()
            },
        )
    }
    async fn auth_publickey(
        &mut self,
        user: &str,
        key: &PublicKey,
    ) -> Result<server::Auth, Self::Error> {
        if user != PUBLIC_CLIENT_USER || self.admission.is_some() {
            return Ok(server::Auth::reject());
        }
        let Some(key) = key.key_data().ed25519() else {
            return Ok(server::Auth::reject());
        };
        let Some(registry) = self.registry.as_ref() else {
            return Ok(server::Auth::reject());
        };
        // russh calls this only after verifying the client signature.
        let Ok(admission) = registry.admit_verified_key(&key.0, now_ms()?) else {
            return Ok(server::Auth::reject());
        };
        if self.authenticated.try_send(admission.clone()).is_err() {
            return Ok(server::Auth::reject());
        }
        self.admission = Some(admission);
        // A live connection must not itself keep the registry owner alive.
        self.registry.take();
        Ok(server::Auth::Accept)
    }
    async fn channel_open_session(
        &mut self,
        channel: Channel<server::Msg>,
        reply: server::ChannelOpenHandle,
        _: &mut server::Session,
    ) -> Result<(), Self::Error> {
        let Some(admission) = self.admission.as_ref() else {
            return Ok(());
        };
        if self.transport.is_closed() || !admission.is_current(now_ms()?) {
            return Ok(());
        }
        let Ok(capacity) = self.capacity.clone().try_acquire_owned() else {
            return Ok(());
        };
        let Ok(queued) = self.incoming.clone().try_reserve_owned() else {
            return Ok(());
        };
        reply.accept().await;
        queued.send(IncomingChannel {
            admission: admission.clone(),
            connection_id: self.connection_id,
            stream: ClientStream::new(channel.into_stream(), capacity),
        });
        Ok(())
    }
}
pub(crate) fn config(key: Arc<PrivateKey>) -> Arc<server::Config> {
    Arc::new(server::Config {
        keys: vec![(*key).clone()],
        methods: russh::MethodSet::from(&[russh::MethodKind::PublicKey][..]),
        max_auth_attempts: MAX_AUTH_ATTEMPTS,
        channel_buffer_size: SSH_CHANNEL_MESSAGES,
        event_buffer_size: SSH_SESSION_MESSAGES,
        window_size: SSH_WINDOW_BYTES,
        maximum_packet_size: SSH_PACKET_BYTES,
        keepalive_interval: Some(KEEPALIVE_INTERVAL),
        keepalive_max: MISSED_KEEPALIVES,
        auth_rejection_time: AUTH_REJECTION_DELAY,
        ..Default::default()
    })
}
