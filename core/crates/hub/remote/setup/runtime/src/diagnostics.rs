use std::fmt;
use yolo_remote_broker::BrokerClientError;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Stage {
    BrokerStartup,
    Provisioning,
    Routing,
}

impl fmt::Display for Stage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}
impl std::error::Error for Stage {}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    BrokerHub,
    BrokerProtocol,
    BrokerSocket,
    BrokerPeer,
    BrokerDeadline,
    BrokerRetirement,
    LocalIo,
    Internal,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Failure {
    pub stage: Option<Stage>,
    pub cause: Cause,
}

/// Arbitrary peer replies, decoder diagnostics and anyhow contexts can contain
/// credentials or commands. Log only closed, locally defined classifications.
pub(crate) fn classify(error: &anyhow::Error) -> Failure {
    let cause = match error.downcast_ref::<BrokerClientError>() {
        Some(BrokerClientError::HubProtocol(_)) => Cause::BrokerHub,
        Some(
            BrokerClientError::Protocol(_)
            | BrokerClientError::IncompatibleProtocol
            | BrokerClientError::RequestMismatch
            | BrokerClientError::UnexpectedResult,
        ) => Cause::BrokerProtocol,
        Some(BrokerClientError::UnsafeSocket(_)) => Cause::BrokerSocket,
        Some(BrokerClientError::Remote { .. }) => Cause::BrokerPeer,
        Some(BrokerClientError::Deadline) => Cause::BrokerDeadline,
        Some(BrokerClientError::Retirement(_)) => Cause::BrokerRetirement,
        None if error.is::<std::io::Error>() => Cause::LocalIo,
        None => Cause::Internal,
    };
    Failure {
        stage: error.downcast_ref::<Stage>().copied(),
        cause,
    }
}
