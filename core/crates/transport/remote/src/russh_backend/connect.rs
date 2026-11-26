use super::*;

impl RusshConnection {
    pub async fn connect(config: SshConnectConfig) -> Result<Self, RemoteError> {
        let (client_config, handler, presented) = connection_parts(&config);
        let address = (config.endpoint.host(), config.endpoint.port());
        let connection = client::connect(client_config, address, handler);
        let session = tokio::time::timeout(config.connect_timeout, connection)
            .await
            .map_err(|_| RemoteError::transport("SSH connection timed out"))?
            .map_err(handler_error)?;
        Self::finish_connect(session, config, presented).await
    }

    pub(super) async fn finish_connect(
        mut session: client::Handle<HostKeyHandler>,
        config: SshConnectConfig,
        presented: Arc<Mutex<Option<HostKey>>>,
    ) -> Result<Self, RemoteError> {
        authentication::authenticate(&mut session, &config).await?;
        let cleanup = config.cleanup.clone().unwrap_or_default();
        let host_key = presented
            .lock()
            .unwrap()
            .clone()
            .ok_or_else(|| RemoteError::transport("server did not present a host key"))?;
        Ok(Self {
            info: ConnectionInfo {
                connection_id: uuid::Uuid::new_v4(),
                endpoint: config.endpoint,
                host_key,
            },
            session: Arc::new(session),
            next_channel_id: AtomicU64::new(FIRST_CHANNEL_ID),
            operation_timeout: config.operation_timeout,
            cleanup,
            authenticated_uid: tokio::sync::OnceCell::new(),
        })
    }
}

pub(super) fn connection_parts(
    config: &SshConnectConfig,
) -> (
    Arc<client::Config>,
    HostKeyHandler,
    Arc<Mutex<Option<HostKey>>>,
) {
    let presented = Arc::new(Mutex::new(None));
    let handler = HostKeyHandler {
        endpoint: config.endpoint.clone(),
        known_hosts: config.known_hosts.clone(),
        presented: Arc::clone(&presented),
    };
    let client_config = Arc::new(client::Config {
        inactivity_timeout: Some(config.inactivity_timeout),
        keepalive_interval: Some(config.keepalive_interval),
        keepalive_max: config.keepalive_max_failures,
        ..Default::default()
    });
    (client_config, handler, presented)
}

pub(super) fn handler_error(error: HandlerError) -> RemoteError {
    match error {
        HandlerError::Remote(error) => error,
        other => ssh_error(other),
    }
}
