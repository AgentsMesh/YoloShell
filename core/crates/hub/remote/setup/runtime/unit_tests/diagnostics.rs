use crate::diagnostics::{Cause, Failure, Stage, classify};
use yolo_remote_broker::BrokerClientError;

const SECRET: &str = "passphrase=private-credential; command=private-command";

#[test]
fn peer_error_fields_and_arbitrary_contexts_never_enter_setup_diagnostics() {
    let cases = [
        (
            BrokerClientError::Remote {
                code: SECRET.into(),
                message: SECRET.into(),
            },
            Cause::BrokerPeer,
        ),
        (
            BrokerClientError::UnsafeSocket(SECRET.into()),
            Cause::BrokerSocket,
        ),
        (
            BrokerClientError::Retirement(SECRET.into()),
            Cause::BrokerRetirement,
        ),
    ];
    for (error, cause) in cases {
        let error = anyhow::Error::new(error)
            .context(Stage::Provisioning)
            .context(SECRET);
        let failure = classify(&error);
        assert_eq!(
            failure,
            Failure {
                stage: Some(Stage::Provisioning),
                cause
            }
        );
        assert!(!format!("{failure:?}").contains(SECRET));
    }
}

#[test]
fn unrecognized_and_io_error_messages_are_never_logged() {
    for (error, cause) in [
        (anyhow::anyhow!(SECRET), Cause::Internal),
        (
            anyhow::Error::new(std::io::Error::other(SECRET)),
            Cause::LocalIo,
        ),
    ] {
        let failure = classify(&error.context(SECRET));
        assert_eq!(failure, Failure { stage: None, cause });
        assert!(!format!("{failure:?}").contains(SECRET));
    }
}
