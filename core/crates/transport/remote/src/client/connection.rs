impl RemoteHost {
    pub async fn connect_with_artifact(
        connection: Arc<dyn MultiplexedSshConnection>,
        known_hosts: &KnownHosts,
        runtime: RemoteRuntimeSpec,
        artifact: &RuntimeArtifact,
    ) -> Result<(Self, RuntimeDeploymentResult), RemoteError> {
        verify_connection(connection.as_ref(), known_hosts)?;
        let deployment = RuntimeInstaller
            .ensure(connection.as_ref(), &runtime, artifact)
            .await?;
        let host = Self::connect(connection, known_hosts, runtime).await?;
        Ok((host, deployment))
    }

    pub async fn connect(
        connection: Arc<dyn MultiplexedSshConnection>,
        known_hosts: &KnownHosts,
        runtime: RemoteRuntimeSpec,
    ) -> Result<Self, RemoteError> {
        verify_connection(connection.as_ref(), known_hosts)?;
        let deadline = RemoteDeadline::operation();
        let (control, status, identity) =
            open_control(connection.as_ref(), &runtime, deadline).await?;
        Ok(Self {
            connection,
            runtime,
            control: Some(control),
            status,
            state: RemoteState::Ready(identity),
        })
    }

    pub fn state(&self) -> &RemoteState {
        &self.state
    }

    pub fn status(&self) -> &HostStatus {
        &self.status
    }

    pub async fn request<T: Serialize, R: DeserializeOwned>(
        &mut self,
        method: &str,
        payload: &T,
    ) -> Result<R, RemoteError> {
        let Some(control) = self.control.as_mut() else {
            return Err(RemoteError::Disconnected);
        };
        let result = control
            .request_until(method, payload, RemoteDeadline::operation())
            .await;
        if result.as_ref().err().is_some_and(|error| {
            error.requires_connection_recovery()
                || matches!(
                    error,
                    RemoteError::Protocol(crate::protocol::ProtocolError::OutcomeUnknown)
                )
        }) {
            // Retire the poisoned channel without retrying the uncertain operation.
            self.mark_transport_lost();
        }
        result
    }

    pub fn mark_transport_lost(&mut self) {
        let previous_connection_id = self.connection.info().connection_id;
        self.control = None;
        self.state = RemoteState::Disconnected {
            previous_connection_id,
        };
    }

    pub async fn reconnect(
        &mut self,
        connection: Arc<dyn MultiplexedSshConnection>,
        known_hosts: &KnownHosts,
    ) -> Result<ReconnectReport, RemoteError> {
        if connection.info().endpoint != self.connection.info().endpoint {
            return Err(RemoteError::InvalidEndpoint(
                "reconnect target differs".into(),
            ));
        }
        verify_connection(connection.as_ref(), known_hosts)?;
        let previous = current_identity(&self.state, &self.status);
        let deadline = RemoteDeadline::operation();
        let (control, status, current) =
            open_control(connection.as_ref(), &self.runtime, deadline).await?;
        let report = ReconnectReport {
            same_boot: previous.boot_id == current.boot_id,
            previous,
            current: current.clone(),
        };
        self.connection = connection;
        self.control = Some(control);
        self.status = status;
        self.state = RemoteState::Ready(current);
        Ok(report)
    }
}

fn current_identity(state: &RemoteState, status: &HostStatus) -> RuntimeIdentity {
    match state {
        RemoteState::Ready(identity) => identity.clone(),
        RemoteState::Disconnected {
            previous_connection_id,
        } => RuntimeIdentity::from_status(*previous_connection_id, status),
    }
}
