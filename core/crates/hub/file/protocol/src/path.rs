use crate::constants::*;
use serde::{Deserialize, Serialize};
use yolo_hub_terminal_protocol::PublicError;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct RelativeFilePath(String);

impl RelativeFilePath {
    pub fn new(value: String) -> Result<Self, PublicError> {
        if !relative(&value) {
            return Err(PublicError::InvalidMetadata);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl TryFrom<String> for RelativeFilePath {
    type Error = PublicError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<RelativeFilePath> for String {
    fn from(value: RelativeFilePath) -> Self {
        value.0
    }
}

pub(crate) fn relative(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_FILE_PATH_BYTES || value.chars().any(char::is_control)
    {
        return false;
    }
    let components = value.split(PATH_SEPARATOR).collect::<Vec<_>>();
    components.len() <= MAX_FILE_PATH_DEPTH
        && components.iter().all(|component| {
            !component.is_empty()
                && component.len() <= MAX_FILE_COMPONENT_BYTES
                && *component != CURRENT_COMPONENT
                && *component != PARENT_COMPONENT
        })
}

pub(crate) fn absolute_root(value: &str) -> bool {
    value.len() <= MAX_FILE_PATH_BYTES && value.strip_prefix(PATH_SEPARATOR).is_some_and(relative)
}

pub(crate) fn root_name(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= MAX_FILE_ROOT_NAME_BYTES
        && !value.chars().any(char::is_control)
}
