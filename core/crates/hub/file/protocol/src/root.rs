use crate::path::{absolute_root, root_name};
use serde::{Deserialize, Serialize};
use yolo_hub_identity::{FileRootEpoch, FileRootId, HubId};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRootAddress {
    pub hub_id: HubId,
    pub root_id: FileRootId,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRootTarget {
    pub address: FileRootAddress,
    pub epoch: FileRootEpoch,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRootFields {
    pub root_id: FileRootId,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "FileRootFields", into = "FileRootFields")]
pub struct FileRootConfiguration(FileRootFields);

impl FileRootConfiguration {
    pub fn new(fields: FileRootFields) -> Result<Self, PublicError> {
        if !root_name(&fields.name) || !absolute_root(&fields.path) {
            return Err(PublicError::InvalidMetadata);
        }
        Ok(Self(fields))
    }
    pub fn fields(&self) -> &FileRootFields {
        &self.0
    }
}
impl TryFrom<FileRootFields> for FileRootConfiguration {
    type Error = PublicError;
    fn try_from(value: FileRootFields) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<FileRootConfiguration> for FileRootFields {
    fn from(value: FileRootConfiguration) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileRootSummaryFields {
    pub target: FileRootTarget,
    pub name: String,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "FileRootSummaryFields", into = "FileRootSummaryFields")]
pub struct FileRootSummary(FileRootSummaryFields);

impl FileRootSummary {
    pub fn new(fields: FileRootSummaryFields) -> Result<Self, PublicError> {
        if !root_name(&fields.name) || !absolute_root(&fields.path) {
            return Err(PublicError::InvalidMetadata);
        }
        Ok(Self(fields))
    }
    pub fn fields(&self) -> &FileRootSummaryFields {
        &self.0
    }
}
impl TryFrom<FileRootSummaryFields> for FileRootSummary {
    type Error = PublicError;
    fn try_from(value: FileRootSummaryFields) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<FileRootSummary> for FileRootSummaryFields {
    fn from(value: FileRootSummary) -> Self {
        value.0
    }
}
