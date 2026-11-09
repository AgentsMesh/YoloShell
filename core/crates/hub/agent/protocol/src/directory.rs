use crate::{MAX_AGENT_WORKING_DIRECTORY_BYTES, ProtocolError};
use serde::{Deserialize, Serialize};

/// Last observed directory on the Agent's origin machine. This is display metadata,
/// never a client-local path or authority to read files or execute a command.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct AgentWorkingDirectory(String);

impl AgentWorkingDirectory {
    pub fn new(path: String) -> Result<Self, ProtocolError> {
        path.try_into()
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for AgentWorkingDirectory {
    type Error = ProtocolError;

    fn try_from(path: String) -> Result<Self, Self::Error> {
        if !std::path::Path::new(&path).is_absolute()
            || path.len() > MAX_AGENT_WORKING_DIRECTORY_BYTES
            || path.chars().any(char::is_control)
        {
            return Err(ProtocolError::InvalidWorkingDirectory);
        }
        Ok(Self(path))
    }
}

impl From<AgentWorkingDirectory> for String {
    fn from(directory: AgentWorkingDirectory) -> Self {
        directory.0
    }
}
