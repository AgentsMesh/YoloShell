use crate::delivery::DeliveryCheckpoint;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

pub const OUTPUT_NAME: &str = "delivery";
pub const RETAINED_NAME: &str = "retained-original";
pub const REJECTED_COPY_NAME: &str = "rejected-copy";
pub const STOP_MESSAGE: &str = "private remount fixture reached durable intent";

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Phase {
    Prepared,
    Committed,
}

impl Phase {
    pub fn checkpoint(self) -> DeliveryCheckpoint {
        match self {
            Self::Prepared => DeliveryCheckpoint::PreparedBeforeRename,
            Self::Committed => DeliveryCheckpoint::PublishedDestinationOpen,
        }
    }
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Record {
    pub phase: Phase,
    pub directory: PathBuf,
    pub candidate_name: PathBuf,
    pub stage_name: PathBuf,
    pub device_before: u64,
    pub receipt_sha256: String,
}

pub fn child(parent: &Path, name: &Path) -> Result<PathBuf> {
    let mut parts = name.components();
    ensure!(
        matches!(parts.next(), Some(Component::Normal(_))) && parts.next().is_none(),
        "remount fixture name must be one normal path component"
    );
    Ok(parent.join(name))
}
