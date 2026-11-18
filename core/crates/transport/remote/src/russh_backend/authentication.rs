use super::super::constants::{
    MAX_SSH_IDENTITY_FILES, SSH_AGENT_FAILURE_CONTEXT, SSH_IDENTITY_FAILURE_CONTEXT,
};
use super::deadline::operation;
use super::deadline::remote_operation;
use super::*;
use russh::keys::PrivateKeyWithHashAlg;
use russh::keys::agent::AgentIdentity;
use std::borrow::Cow;

pub(super) async fn authenticate(
    session: &mut client::Handle<HostKeyHandler>,
    config: &SshConnectConfig,
) -> Result<(), RemoteError> {
    let mut identity_error = None;
    for identity in config.identities.iter().take(MAX_SSH_IDENTITY_FILES) {
        let secret = match identity_secret(identity) {
            Ok(secret) => secret,
            Err(error) => {
                identity_error = Some(error);
                continue;
            }
        };
        let key = match russh::keys::decode_secret_key(&secret, identity.passphrase()) {
            Ok(key) => key,
            Err(error) => {
                identity_error = Some(ssh_error(error));
                continue;
            }
        };
        let hash = operation(config.operation_timeout, session.best_supported_rsa_hash())
            .await?
            .flatten();
        let result = operation(
            config.operation_timeout,
            session.authenticate_publickey(
                config.endpoint.user(),
                PrivateKeyWithHashAlg::new(Arc::new(key), hash),
            ),
        )
        .await?;
        if result.success() {
            return Ok(());
        }
    }
    let agent_result = authenticate_agent(session, config).await;
    if matches!(agent_result, Ok(true)) {
        return Ok(());
    }
    let context = if let Err(error) = agent_result {
        format!("; {SSH_AGENT_FAILURE_CONTEXT}: {error}")
    } else if let Some(error) = identity_error {
        format!("; {SSH_IDENTITY_FAILURE_CONTEXT}: {error}")
    } else {
        String::new()
    };
    Err(RemoteError::AuthenticationFailed(format!(
        "{}{context}",
        config.endpoint.authority()
    )))
}

fn identity_secret(identity: &IdentityFile) -> Result<Cow<'_, str>, RemoteError> {
    match identity.material() {
        Some(material) => super::identity_file::material_text(material).map(Cow::Borrowed),
        None => super::super::local_file::read_identity(identity.path()).map(Cow::Owned),
    }
}

async fn authenticate_agent(
    session: &mut client::Handle<HostKeyHandler>,
    config: &SshConnectConfig,
) -> Result<bool, RemoteError> {
    let connected = match &config.agent {
        AgentSource::Disabled => return Ok(false),
        AgentSource::Environment => {
            operation(
                config.operation_timeout,
                super::agent_identities::connect_env(),
            )
            .await?
        }
        AgentSource::Socket(path) => {
            operation(
                config.operation_timeout,
                super::agent_identities::connect_uds(path),
            )
            .await?
        }
    };
    let (mut agent, identities) = remote_operation(
        config.operation_timeout,
        super::agent_identities::request_bounded(connected),
    )
    .await?;
    for identity in identities {
        let hash = operation(config.operation_timeout, session.best_supported_rsa_hash())
            .await?
            .flatten();
        let result = match identity {
            AgentIdentity::PublicKey { key, .. } => {
                operation(
                    config.operation_timeout,
                    session.authenticate_publickey_with(
                        config.endpoint.user(),
                        key,
                        hash,
                        &mut agent,
                    ),
                )
                .await?
            }
            AgentIdentity::Certificate { certificate, .. } => {
                operation(
                    config.operation_timeout,
                    session.authenticate_certificate_with(
                        config.endpoint.user(),
                        certificate,
                        hash,
                        &mut agent,
                    ),
                )
                .await?
            }
        };
        if result.success() {
            return Ok(true);
        }
    }
    Ok(false)
}
