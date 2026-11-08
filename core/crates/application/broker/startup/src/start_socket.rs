use anyhow::{Context, Result};
use std::path::Path;

#[cfg(all(test, feature = "yolo-integration-tests"))]
#[path = "../integration_tests/start_socket_tests.rs"]
mod tests;
use yolo_remote_broker::validate_broker_socket;

pub(super) fn validate_existing(path: &Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(_) => validate_broker_socket(path),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error).context("inspect existing broker socket"),
    }
}
