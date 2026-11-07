use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::os::fd::AsFd;
use yolo_durable_identity::DurableObjectIdentity;
use yolo_release_filesystem::DirectoryHandle;

const DURABLE_DIRECTORY_ERROR: &str =
    "delivery output requires an APFS directory with persistent filesystem identity";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct DeliveryObjectIdentity(DurableObjectIdentity);

impl DeliveryObjectIdentity {
    pub fn capture(directory: &DirectoryHandle) -> Result<Self> {
        DurableObjectIdentity::capture(directory.retained_file().as_fd())
            .map(Self)
            .context(DURABLE_DIRECTORY_ERROR)
    }

    pub fn matches(&self, directory: &DirectoryHandle) -> Result<bool> {
        Ok(*self == Self::capture(directory)?)
    }
}
