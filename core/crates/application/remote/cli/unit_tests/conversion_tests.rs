use super::*;
use crate::{conversion, error_constants as errors};
use yolo_broker_supervision_state::CredentialSource;

fn options() -> ServerOptions {
    ServerOptions {
        config: std::path::Path::new(fixture::TEST_HOME).join(fixture::CONFIG_FILE),
        state_dir: std::path::Path::new(fixture::TEST_HOME).join(fixture::STATE_DIRECTORY),
        expected_revision: None,
        expected_anchor: None,
        expected_authority: None,
        expected_startup_deadline: None,
        authority_document: None,
        authority_lock: None,
        expected_profile_sha256: None,
        identity_passphrases_stdin: false,
    }
}

#[test]
fn child_conversion_requires_each_authority_field_and_preserves_original_deadline() {
    let mut options = options();
    let failure = |options: &ServerOptions, message| {
        assert_eq!(
            conversion::child(options.clone()).unwrap_err().to_string(),
            message
        );
    };
    failure(&options, errors::REVISION_REQUIRED);
    options.expected_revision = Some(fixture::TEST_CONFIG_REVISION.into());
    failure(&options, errors::ANCHOR_REQUIRED);
    options.expected_anchor = Some(fixture::TEST_STATE_ANCHOR.into());
    failure(&options, errors::AUTHORITY_REQUIRED);
    options.expected_authority = Some(uuid::Uuid::nil().to_string());
    failure(&options, errors::DEADLINE_REQUIRED);
    let deadline = yolo_remote_broker::BrokerLaunchDeadline::capture(
        tokio::time::Instant::now() + fixture::START_TIMEOUT,
    )
    .unwrap();
    options.expected_startup_deadline = Some(deadline.clone());
    let launch = conversion::child(options.clone()).unwrap();
    assert_eq!(launch.config(), options.config);
    assert_eq!(launch.state_dir(), options.state_dir);
    assert_eq!(launch.startup_deadline().to_string(), deadline.to_string());
    assert_eq!(launch.config_revision(), fixture::TEST_CONFIG_REVISION);
    assert_eq!(
        launch.state_anchor().to_string(),
        fixture::TEST_STATE_ANCHOR
    );
    assert_eq!(
        launch.authority().to_string(),
        uuid::Uuid::nil().to_string()
    );
    let mut invalid_revision = options.clone();
    invalid_revision.expected_revision = Some(fixture::INVALID_FENCED_INPUT.into());
    assert!(conversion::child(invalid_revision).is_err());
    let mut invalid_anchor = options.clone();
    invalid_anchor.expected_anchor = Some(fixture::INVALID_FENCED_INPUT.into());
    assert!(conversion::child(invalid_anchor).is_err());
    options.expected_authority = Some(fixture::INVALID_FENCED_INPUT.into());
    assert!(conversion::child(options).is_err());
}

#[test]
fn start_conversion_requires_profile_authority_and_only_describes_credential_source() {
    let mut options = options();
    for message in [
        errors::DOCUMENT_REQUIRED,
        errors::LOCK_REQUIRED,
        errors::PROFILE_REQUIRED,
    ] {
        assert_eq!(
            conversion::start(options.clone()).unwrap_err().to_string(),
            message
        );
        match message {
            errors::DOCUMENT_REQUIRED => options.authority_document = Some(options.config.clone()),
            errors::LOCK_REQUIRED => options.authority_lock = Some(options.state_dir.clone()),
            _ => {
                options.expected_profile_sha256 =
                    Some(fixture::TEST_PROFILE_AUTHORITY_SHA256.into())
            }
        }
    }
    assert_eq!(conversion::credentials(&options), CredentialSource::None);
    options.identity_passphrases_stdin = true;
    assert_eq!(
        conversion::credentials(&options),
        CredentialSource::PrivateStdin
    );
    let (start, authority) = conversion::start(options.clone()).unwrap();
    assert_eq!(start.config_source, options.config);
    assert_eq!(start.broker_state, options.state_dir);
    assert_eq!(authority.document, options.authority_document.unwrap());
    assert_eq!(authority.lock, options.authority_lock.unwrap());
    assert_eq!(
        authority.expected_sha256,
        fixture::TEST_PROFILE_AUTHORITY_SHA256
    );
}
