use super::*;

pub struct SshConnectConfig {
    pub endpoint: RemoteEndpoint,
    pub known_hosts: KnownHosts,
    pub(super) identities: Vec<IdentityFile>,
    pub(super) agent: AgentSource,
    pub(super) connect_timeout: Duration,
    pub(super) inactivity_timeout: Duration,
    pub(super) keepalive_interval: Duration,
    pub(super) keepalive_max_failures: usize,
    pub(super) operation_timeout: Duration,
    pub(super) cleanup: Option<RemoteCleanupRegistry>,
}

pub(super) enum AgentSource {
    Disabled,
    Environment,
    Socket(PathBuf),
}

impl SshConnectConfig {
    pub fn new(endpoint: RemoteEndpoint, known_hosts: KnownHosts) -> Self {
        Self {
            endpoint,
            known_hosts,
            identities: Vec::new(),
            agent: AgentSource::Environment,
            connect_timeout: SSH_CONNECT_TIMEOUT,
            inactivity_timeout: SSH_INACTIVITY_TIMEOUT,
            keepalive_interval: SSH_KEEPALIVE_INTERVAL,
            keepalive_max_failures: SSH_KEEPALIVE_MAX_FAILURES,
            operation_timeout: SSH_OPERATION_TIMEOUT,
            cleanup: None,
        }
    }

    pub fn with_identity(mut self, identity: IdentityFile) -> Self {
        self.identities.push(identity);
        self
    }

    #[cfg(test)]
    pub(crate) fn identities(&self) -> &[IdentityFile] {
        &self.identities
    }

    #[cfg(feature = "broker-test-support")]
    #[doc(hidden)]
    pub fn identities_for_broker_test(&self) -> &[IdentityFile] {
        &self.identities
    }

    pub fn use_agent(mut self, use_agent: bool) -> Self {
        self.agent = if use_agent {
            AgentSource::Environment
        } else {
            AgentSource::Disabled
        };
        self
    }

    pub fn with_agent_socket(mut self, path: impl Into<PathBuf>) -> Self {
        self.agent = AgentSource::Socket(path.into());
        self
    }

    pub fn with_timeouts(mut self, connect: Duration, inactivity: Duration) -> Self {
        self.connect_timeout = connect;
        self.inactivity_timeout = inactivity;
        self
    }

    pub fn with_operation_timeout(mut self, timeout: Duration) -> Self {
        self.operation_timeout = timeout;
        self
    }

    pub fn with_cleanup_registry(mut self, cleanup: RemoteCleanupRegistry) -> Self {
        self.cleanup = Some(cleanup);
        self
    }

    #[cfg(test)]
    pub(crate) fn with_keepalive_for_test(
        mut self,
        interval: Duration,
        max_failures: usize,
    ) -> Self {
        self.keepalive_interval = interval;
        self.keepalive_max_failures = max_failures;
        self
    }
}
