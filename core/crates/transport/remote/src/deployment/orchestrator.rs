use super::super::constants::{
    ARCHITECTURE_ARGUMENT, ARCHITECTURE_PROGRAM, PLATFORM_ARGUMENT, RUNTIME_VERSION_ARGUMENT,
    SUCCESS_EXIT_STATUS,
};
use super::super::transport::validate_channel;
use super::super::{
    ChannelPurpose, CommandPolicy, CommandRequest, MultiplexedSshConnection, RemoteError,
    RemoteRuntimeSpec,
};
use super::RuntimeArtifact;
use super::plan::DeploymentPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// File bytes are fsynced and activation is atomic. SFTP cannot fsync the parent directory, so
/// crash durability of the selected directory entry remains a server-filesystem guarantee.
pub enum RuntimeDeploymentResult {
    Installed,
    Current,
    Upgraded,
}

#[derive(Debug, Default)]
pub struct RuntimeInstaller;

impl RuntimeInstaller {
    pub async fn ensure(
        &self,
        connection: &dyn MultiplexedSshConnection,
        runtime: &RemoteRuntimeSpec,
        artifact: &RuntimeArtifact,
    ) -> Result<RuntimeDeploymentResult, RemoteError> {
        artifact.validate_integrity()?;
        verify_platform(connection, artifact).await?;
        verify_architecture(connection, artifact).await?;
        let plan = DeploymentPlan::new(runtime, artifact)?;
        let stage_request = plan.stage(artifact);
        let staged = connection.stage_runtime(&stage_request).await?;
        validate_channel(
            staged.identity,
            connection.info().connection_id,
            ChannelPurpose::RuntimeDeployment,
        )?;
        verify_version(connection, plan.release_path(), artifact.version()).await?;
        if staged.value.already_active {
            return Ok(RuntimeDeploymentResult::Current);
        }
        let activated = connection
            .activate_runtime(&plan.activation(artifact))
            .await?;
        validate_channel(
            activated.identity,
            connection.info().connection_id,
            ChannelPurpose::RuntimeDeployment,
        )?;
        Ok(if staged.value.had_previous_runtime {
            RuntimeDeploymentResult::Upgraded
        } else {
            RuntimeDeploymentResult::Installed
        })
    }
}

async fn verify_platform(
    connection: &dyn MultiplexedSshConnection,
    artifact: &RuntimeArtifact,
) -> Result<(), RemoteError> {
    let output = run(
        connection,
        ARCHITECTURE_PROGRAM,
        vec![PLATFORM_ARGUMENT.into()],
    )
    .await?;
    if artifact.platform().matches_remote(output.trim()) {
        Ok(())
    } else {
        Err(RemoteError::RuntimeIncompatible(format!(
            "remote platform {} does not match artifact {}",
            output.trim(),
            artifact.platform().path_name()
        )))
    }
}

async fn verify_architecture(
    connection: &dyn MultiplexedSshConnection,
    artifact: &RuntimeArtifact,
) -> Result<(), RemoteError> {
    let output = run(
        connection,
        ARCHITECTURE_PROGRAM,
        vec![ARCHITECTURE_ARGUMENT.into()],
    )
    .await?;
    if artifact.architecture().matches_remote(output.trim()) {
        Ok(())
    } else {
        Err(RemoteError::RuntimeIncompatible(format!(
            "remote architecture {} does not match artifact {}",
            output.trim(),
            artifact.architecture().path_name()
        )))
    }
}

async fn verify_version(
    connection: &dyn MultiplexedSshConnection,
    program: &str,
    expected: &str,
) -> Result<(), RemoteError> {
    let output = run(connection, program, vec![RUNTIME_VERSION_ARGUMENT.into()]).await?;
    if output.split_whitespace().last() == Some(expected) {
        Ok(())
    } else {
        Err(RemoteError::RuntimeIncompatible(
            "uploaded runtime reported a different version".into(),
        ))
    }
}

async fn run(
    connection: &dyn MultiplexedSshConnection,
    program: &str,
    args: Vec<String>,
) -> Result<String, RemoteError> {
    let policy = CommandPolicy::new([program.to_owned()])?;
    let command = policy.authorize(CommandRequest::new(program, args))?;
    let outcome = connection.execute(&command).await?;
    validate_channel(
        outcome.identity,
        connection.info().connection_id,
        ChannelPurpose::Command,
    )?;
    let output = outcome.value;
    output.validate(&command)?;
    if output.timed_out || output.truncated || output.exit_status != Some(SUCCESS_EXIT_STATUS) {
        return Err(RemoteError::RuntimeDeployment(
            "runtime probe did not complete successfully".into(),
        ));
    }
    String::from_utf8(output.stdout)
        .map_err(|_| RemoteError::RuntimeDeployment("runtime probe returned invalid UTF-8".into()))
}
