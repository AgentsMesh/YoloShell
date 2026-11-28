use super::BrokerConnectionPool;
use super::connection::ObservedConnection;
use super::constants::{INITIAL_RECONNECT_DELAY, MAX_RECONNECT_DELAY};
use super::timing::RUNTIME_OPEN_TIMEOUT;
use crate::remote::{
    BoxedRemoteChannel, ChannelPurpose, OPERATION_TIMEOUT_MESSAGE, OpenedRemoteChannel,
    RemoteDeadline, RemoteError, RuntimeChannelRequest, RuntimeRole, probe_runtime_channel,
};
use uuid::Uuid;

pub(super) const RUNTIME_OPEN_TIMEOUT_MESSAGE: &str = "remote runtime setup timed out";

struct OpenAttempt<'a> {
    pool: &'a BrokerConnectionPool,
    observed: Option<ObservedConnection>,
}

impl<'a> OpenAttempt<'a> {
    fn new(pool: &'a BrokerConnectionPool) -> Self {
        Self {
            pool,
            observed: None,
        }
    }

    fn observe(&mut self, observed: &ObservedConnection) {
        self.observed = Some(observed.clone());
    }

    fn release(&mut self) {
        self.observed = None;
    }
}

impl Drop for OpenAttempt<'_> {
    fn drop(&mut self) {
        if let Some(observed) = &self.observed {
            self.pool.retire(observed);
        }
    }
}

impl BrokerConnectionPool {
    pub async fn open_runtime(
        &self,
        role: RuntimeRole,
    ) -> Result<yolo_runtime_rpc::RuntimeConnection, RemoteError> {
        let mut attempt = OpenAttempt::new(self);
        tokio::time::timeout(
            RUNTIME_OPEN_TIMEOUT,
            self.open_runtime_until(role, &mut attempt),
        )
        .await
        .map_err(|_| RemoteError::RuntimeDeployment(RUNTIME_OPEN_TIMEOUT_MESSAGE.into()))?
    }

    async fn open_runtime_until(
        &self,
        role: RuntimeRole,
        attempt: &mut OpenAttempt<'_>,
    ) -> Result<yolo_runtime_rpc::RuntimeConnection, RemoteError> {
        let mut delay = INITIAL_RECONNECT_DELAY;
        loop {
            let observed = self.bootstrap_connection().await?;
            attempt.observe(&observed);
            let open_deadline = RemoteDeadline::operation();
            let opened = open_deadline
                .run(self.open_on(&observed, role, open_deadline))
                .await;
            match opened {
                Ok(channel) if self.is_retired(&observed) => {
                    drop(channel);
                    attempt.release();
                    tokio::time::sleep(delay).await;
                    delay = next_delay(delay);
                }
                Ok(channel) => {
                    attempt.release();
                    return Ok(channel);
                }
                Err(error) if channel_open_timed_out(&error) => return Err(error),
                Err(error) if runtime_probe_retryable(&error) => {
                    self.invalidate(&observed).await;
                    attempt.release();
                    tokio::time::sleep(delay).await;
                    delay = next_delay(delay);
                }
                Err(error) => {
                    attempt.release();
                    return Err(error);
                }
            }
        }
    }

    pub(super) async fn probe_connection(
        &self,
        observed: &ObservedConnection,
    ) -> Result<(), RemoteError> {
        let deadline = RemoteDeadline::operation();
        let channel = deadline
            .run(self.open_on(observed, RuntimeRole::Control, deadline))
            .await?;
        drop(channel);
        Ok(())
    }

    async fn open_on(
        &self,
        observed: &ObservedConnection,
        role: RuntimeRole,
        deadline: RemoteDeadline,
    ) -> Result<yolo_runtime_rpc::RuntimeConnection, RemoteError> {
        let request = match role {
            RuntimeRole::Control => RuntimeChannelRequest::control(self.runtime().clone()),
            RuntimeRole::Attachment => RuntimeChannelRequest::attachment(self.runtime().clone()),
        };
        let opened = observed.connection().open_runtime(&request).await?;
        let stream = validate_opened(observed.id(), role, opened)?;
        probe_runtime_channel(stream, self.runtime(), deadline)
            .await
            .map_err(|error| {
                // Only read-only HELLO/status probes have run. Preserve their
                // absolute timeout so an expired probe cannot start a retry.
                if matches!(
                    error,
                    RemoteError::Protocol(crate::protocol::ProtocolError::OutcomeUnknown)
                ) && tokio::time::Instant::now() >= deadline.instant()
                {
                    RemoteError::transport(OPERATION_TIMEOUT_MESSAGE)
                } else {
                    error
                }
            })
    }
}

fn validate_opened(
    connection_id: Uuid,
    role: RuntimeRole,
    opened: OpenedRemoteChannel,
) -> Result<BoxedRemoteChannel, RemoteError> {
    let purpose = match role {
        RuntimeRole::Control => ChannelPurpose::RuntimeControl,
        RuntimeRole::Attachment => ChannelPurpose::TerminalAttachment,
    };
    if opened.identity.connection_id != connection_id || opened.identity.purpose != purpose {
        return Err(RemoteError::ChannelMismatch);
    }
    Ok(opened.stream)
}

pub(super) fn connection_lost(error: &RemoteError) -> bool {
    error.requires_connection_recovery()
}

/// This path has issued only HELLO and HOST_STATUS, never a business operation.
pub(super) fn runtime_probe_retryable(error: &RemoteError) -> bool {
    connection_lost(error)
        || matches!(
            error,
            RemoteError::Protocol(crate::protocol::ProtocolError::OutcomeUnknown)
        )
}

pub(super) fn channel_open_timed_out(error: &RemoteError) -> bool {
    matches!(error, RemoteError::Transport(message) if message == OPERATION_TIMEOUT_MESSAGE)
}

pub(super) fn next_delay(current: std::time::Duration) -> std::time::Duration {
    current.saturating_mul(2).min(MAX_RECONNECT_DELAY)
}
