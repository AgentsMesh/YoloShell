use crate::{BuildImageError, constants::*};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildImageFields {
    pub schema_version: u16,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "BuildImageFields", into = "BuildImageFields")]
pub struct BuildImage(BuildImageFields);

impl BuildImage {
    pub fn new(sha256: String, size_bytes: u64) -> Result<Self, BuildImageError> {
        Self::try_from(BuildImageFields {
            schema_version: BUILD_IMAGE_SCHEMA_VERSION,
            sha256,
            size_bytes,
        })
    }
    pub fn fields(&self) -> &BuildImageFields {
        &self.0
    }
    pub fn sha256(&self) -> &str {
        &self.0.sha256
    }
    pub fn size_bytes(&self) -> u64 {
        self.0.size_bytes
    }
}

impl TryFrom<BuildImageFields> for BuildImage {
    type Error = BuildImageError;
    fn try_from(fields: BuildImageFields) -> Result<Self, Self::Error> {
        if fields.schema_version != BUILD_IMAGE_SCHEMA_VERSION {
            return Err(BuildImageError::UnsupportedSchema);
        }
        if fields.sha256.len() != SHA256_HEX_BYTES
            || !fields
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(BuildImageError::InvalidDigest);
        }
        if fields.size_bytes == u64::default() || fields.size_bytes > MAX_EXECUTABLE_IMAGE_BYTES {
            return Err(BuildImageError::InvalidSize);
        }
        Ok(Self(fields))
    }
}

impl From<BuildImage> for BuildImageFields {
    fn from(image: BuildImage) -> Self {
        image.0
    }
}
