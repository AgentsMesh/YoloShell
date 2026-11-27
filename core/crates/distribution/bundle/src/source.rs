use crate::{BundleDescriptor, BundleError, constants::MAX_SOURCE_PATH_BYTES};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A local immutable bundle locator; executable bytes never enter management frames.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SourceFields", into = "SourceFields")]
pub struct BundleSource(SourceFields);
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceFields {
    root: PathBuf,
    descriptor: BundleDescriptor,
}

impl BundleSource {
    pub fn new(root: PathBuf, descriptor: BundleDescriptor) -> Result<Self, BundleError> {
        SourceFields { root, descriptor }.try_into()
    }
    pub fn root(&self) -> &Path {
        &self.0.root
    }
    pub fn descriptor(&self) -> &BundleDescriptor {
        &self.0.descriptor
    }
}
impl TryFrom<SourceFields> for BundleSource {
    type Error = BundleError;
    fn try_from(fields: SourceFields) -> Result<Self, Self::Error> {
        let text = fields.root.to_str().ok_or(BundleError::SourcePath)?;
        let relative = text.strip_prefix('/').ok_or(BundleError::SourcePath)?;
        if text.len() > MAX_SOURCE_PATH_BYTES
            || text.chars().any(char::is_control)
            || relative.is_empty()
            || !relative
                .split('/')
                .all(|part| !part.is_empty() && part != "." && part != "..")
        {
            return Err(BundleError::SourcePath);
        }
        Ok(Self(fields))
    }
}
impl From<BundleSource> for SourceFields {
    fn from(value: BundleSource) -> Self {
        value.0
    }
}
