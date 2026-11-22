use super::BrokerConnectionPool;
use super::connection::ObservedConnection;
use super::connection_runtime::connection_lost;
use super::protocol::{BrokerCommandRequest, BrokerFileRequest, BrokerResult};
use crate::remote::{
    ChannelIdentity, ChannelPurpose, CommandPolicy, CommandRequest, FileReadPolicy,
    FileReadRequest, RemoteError,
};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

impl BrokerConnectionPool {
    pub async fn working_directory(&self) -> Result<BrokerResult, RemoteError> {
        let observed = self.observed_connection().await?;
        let connection_id = observed.id();
        let outcome = observed
            .connection()
            .canonicalize(Path::new(super::constants::REMOTE_LOGIN_DIRECTORY))
            .await;
        let outcome = match outcome {
            Ok(value) => value,
            Err(error) => {
                self.invalidate_on_loss(&observed, &error).await;
                return Err(error);
            }
        };
        validate_identity(outcome.identity, connection_id, ChannelPurpose::FileRead)?;
        if !crate::remote::strict_absolute(&outcome.value) {
            return Err(RemoteError::PathDenied(outcome.value));
        }
        Ok(BrokerResult::WorkingDirectory {
            connection_id,
            path: outcome.value,
        })
    }

    pub async fn run_command(
        &self,
        policy: &CommandPolicy,
        request: BrokerCommandRequest,
    ) -> Result<BrokerResult, RemoteError> {
        let mut command = CommandRequest::new(request.program, request.args).with_limits(
            Duration::from_millis(request.timeout_millis),
            request.output_limit,
        );
        if let Some(cwd) = request.cwd {
            command = command.with_cwd(cwd);
        }
        let command = policy.authorize(command)?;
        let observed = self.observed_connection().await?;
        let connection_id = observed.id();
        let outcome = observed.connection().execute(&command).await;
        let outcome = match outcome {
            Ok(value) => value,
            Err(error) => {
                self.invalidate_on_loss(&observed, &error).await;
                return Err(error);
            }
        };
        validate_identity(outcome.identity, connection_id, ChannelPurpose::Command)?;
        outcome.value.validate(&command)?;
        Ok(BrokerResult::Command {
            connection_id,
            output: outcome.value.into(),
        })
    }

    pub async fn read_file(
        &self,
        policy: &FileReadPolicy,
        request: BrokerFileRequest,
    ) -> Result<BrokerResult, RemoteError> {
        let request = FileReadRequest::new(request.path, request.offset, request.length);
        policy.preflight(&request)?;
        let observed = self.observed_connection().await?;
        let connection_id = observed.id();
        let canonical = observed.connection().canonicalize(request.path()).await;
        let canonical = match canonical {
            Ok(value) => value,
            Err(error) => {
                self.invalidate_on_loss(&observed, &error).await;
                return Err(error);
            }
        };
        validate_identity(canonical.identity, connection_id, ChannelPurpose::FileRead)?;
        let authorized = policy.authorize(request, canonical.value)?;
        let outcome = observed.connection().read_file(&authorized).await;
        let outcome = match outcome {
            Ok(value) => value,
            Err(error) => {
                self.invalidate_on_loss(&observed, &error).await;
                return Err(error);
            }
        };
        validate_identity(outcome.identity, connection_id, ChannelPurpose::FileRead)?;
        outcome.value.validate(&authorized)?;
        Ok(BrokerResult::FileRead {
            connection_id,
            result: outcome.value.into(),
        })
    }

    async fn invalidate_on_loss(&self, observed: &ObservedConnection, error: &RemoteError) {
        if connection_lost(error) {
            self.invalidate(observed).await;
        }
    }
}

fn validate_identity(
    identity: ChannelIdentity,
    connection_id: Uuid,
    purpose: ChannelPurpose,
) -> Result<(), RemoteError> {
    if identity.connection_id != connection_id || identity.purpose != purpose {
        return Err(RemoteError::ChannelMismatch);
    }
    Ok(())
}
