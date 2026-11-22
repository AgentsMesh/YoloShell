use crate::{ServerOptions, error_constants as errors, options::RetireOptions};
use anyhow::{Context, Result};
use yolo_broker_startup::{ProfileAuthorityRequest, RetireRequest, StartRequest};
use yolo_broker_supervision_state::{ChildLaunch, CredentialSource};

pub(super) fn credentials(options: &ServerOptions) -> CredentialSource {
    if options.identity_passphrases_stdin {
        CredentialSource::PrivateStdin
    } else {
        CredentialSource::None
    }
}

pub(super) fn child(options: ServerOptions) -> Result<ChildLaunch> {
    ChildLaunch::new(
        options.config,
        options.state_dir,
        options
            .expected_revision
            .context(errors::REVISION_REQUIRED)?,
        options
            .expected_anchor
            .context(errors::ANCHOR_REQUIRED)?
            .parse()?,
        options
            .expected_authority
            .context(errors::AUTHORITY_REQUIRED)?
            .parse()?,
        options
            .expected_startup_deadline
            .context(errors::DEADLINE_REQUIRED)?,
    )
}

pub(super) fn start(options: ServerOptions) -> Result<(StartRequest, ProfileAuthorityRequest)> {
    Ok((
        StartRequest {
            config_source: options.config,
            broker_state: options.state_dir,
        },
        ProfileAuthorityRequest {
            document: options
                .authority_document
                .context(errors::DOCUMENT_REQUIRED)?,
            lock: options.authority_lock.context(errors::LOCK_REQUIRED)?,
            expected_sha256: options
                .expected_profile_sha256
                .context(errors::PROFILE_REQUIRED)?,
        },
    ))
}

pub(super) fn retire(options: RetireOptions) -> RetireRequest {
    RetireRequest {
        broker_state: options.state_dir,
        expected_host: options.expected_host,
        authority_document: options.authority_document,
        authority_lock: options.authority_lock,
        expected_mutation_id: options.expected_mutation_id,
    }
}
