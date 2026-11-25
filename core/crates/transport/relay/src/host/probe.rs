use super::observation_interrupted;
use crate::client::ClientError;
use crate::model::HostStatus;

pub(super) fn poll_host_status(
    result: Result<HostStatus, ClientError>,
) -> Result<bool, ClientError> {
    match result {
        Ok(_) => Ok(true),
        Err(error) if observation_interrupted(&error) => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[cfg(feature = "yolo-unit-tests")]
#[path = "../../unit_tests/host/probe_inline_tests.rs"]
mod tests;
