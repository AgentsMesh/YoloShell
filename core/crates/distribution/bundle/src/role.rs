use crate::constants::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactRole {
    Runtime,
}

impl ArtifactRole {
    pub const ALL: [Self; 1] = [Self::Runtime];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Runtime => RUNTIME_ROLE,
        }
    }

    pub fn executable_name(self) -> &'static str {
        match self {
            Self::Runtime => RUNTIME_EXECUTABLE,
        }
    }

    pub fn relative_path(self) -> &'static str {
        match self {
            Self::Runtime => RUNTIME_RELATIVE_PATH,
        }
    }
}
