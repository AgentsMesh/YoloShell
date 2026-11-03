use super::fixture;
use super::process_handoff_protocol::runtime_response;
use russh::keys::ssh_key::LineEnding;
use russh::keys::{Algorithm, PrivateKey, PublicKey};
use russh::server::{Auth, Msg, Session};
use russh::{Channel, ChannelId};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub struct ProcessSshServer {
    pub port: u16,
    pub identity: PathBuf,
    pub known_hosts: PathBuf,
    listener: tokio::task::JoinHandle<()>,
}

impl ProcessSshServer {
    pub async fn start(root: &Path) -> Self {
        let mut random = russh::keys::key::safe_rng();
        let host_key = PrivateKey::random(&mut random, Algorithm::Ed25519).unwrap();
        let user_key = PrivateKey::random(&mut random, Algorithm::Ed25519).unwrap();
        let identity = root.join(fixture::PROCESS_IDENTITY_FILE);
        user_key
            .write_openssh_file(&identity, LineEnding::LF)
            .unwrap();
        set_mode(&identity, fixture::PRIVATE_FILE_MODE);
        let socket = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .unwrap();
        let port = socket.local_addr().unwrap().port();
        let known_hosts = root.join(fixture::PROCESS_KNOWN_HOSTS_FILE);
        let public_host = host_key.public_key().to_openssh().unwrap();
        std::fs::write(
            &known_hosts,
            format!("[{}]:{port} {public_host}\n", fixture::LOOPBACK_HOST),
        )
        .unwrap();
        set_mode(&known_hosts, fixture::TRUSTED_FILE_MODE);
        let config = Arc::new(russh::server::Config {
            auth_rejection_time: std::time::Duration::ZERO,
            auth_rejection_time_initial: Some(std::time::Duration::ZERO),
            keys: vec![host_key],
            ..Default::default()
        });
        let accepted = user_key.public_key().clone();
        let listener = tokio::spawn(accept_connections(socket, config, accepted));
        Self {
            port,
            identity,
            known_hosts,
            listener,
        }
    }
}

impl Drop for ProcessSshServer {
    fn drop(&mut self) {
        self.listener.abort();
    }
}

async fn accept_connections(
    listener: tokio::net::TcpListener,
    config: Arc<russh::server::Config>,
    accepted: PublicKey,
) {
    while let Ok((stream, _)) = listener.accept().await {
        let config = Arc::clone(&config);
        let server = ProbeServer::new(accepted.clone());
        tokio::spawn(async move {
            let running = russh::server::run_stream(config, stream, server)
                .await
                .unwrap();
            let _ = running.await;
        });
    }
}

struct ProbeServer {
    accepted: PublicKey,
    channels: HashMap<ChannelId, Channel<Msg>>,
    tasks: tokio::task::JoinSet<()>,
}

impl ProbeServer {
    fn new(accepted: PublicKey) -> Self {
        Self {
            accepted,
            channels: HashMap::new(),
            tasks: tokio::task::JoinSet::new(),
        }
    }
}

impl russh::server::Handler for ProbeServer {
    type Error = russh::Error;

    async fn auth_publickey(&mut self, _: &str, key: &PublicKey) -> Result<Auth, Self::Error> {
        Ok(if key == &self.accepted {
            Auth::Accept
        } else {
            Auth::reject()
        })
    }

    async fn channel_open_session(
        &mut self,
        channel: Channel<Msg>,
        reply: russh::server::ChannelOpenHandle,
        _: &mut Session,
    ) -> Result<(), Self::Error> {
        self.channels.insert(channel.id(), channel);
        reply.accept().await;
        Ok(())
    }

    async fn exec_request(
        &mut self,
        channel: ChannelId,
        _: &[u8],
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        let stream = self.channels.remove(&channel).unwrap().into_stream();
        session.channel_success(channel)?;
        self.tasks.spawn(async move {
            let _ = yolo_capnp_rpc::serve(
                stream,
                yolo_capnp_rpc::Protocol::Endpoint,
                Arc::new(RuntimeService),
            )
            .await;
        });
        Ok(())
    }

    async fn channel_eof(
        &mut self,
        channel: ChannelId,
        session: &mut Session,
    ) -> Result<(), Self::Error> {
        session.eof(channel)?;
        session.close(channel)
    }
}

struct RuntimeService;
impl yolo_capnp_rpc::Service for RuntimeService {
    fn call(
        &self,
        method: yolo_capnp_rpc::Method,
        value: yolo_capnp_rpc::Value,
    ) -> yolo_capnp_rpc::Future<'_, yolo_capnp_rpc::Reply> {
        Box::pin(async move {
            if method != yolo_capnp_rpc::Method::Call {
                return Err(yolo_capnp_rpc::Error::Invalid);
            }
            let request = yolo_capnp_codec::from_value(value)?;
            let response = runtime_response(&request);
            Ok(yolo_capnp_rpc::Reply::value(yolo_capnp_codec::to_value(
                &response,
            )?))
        })
    }
}

fn set_mode(path: &Path, mode: u32) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}
