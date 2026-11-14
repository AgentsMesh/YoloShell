use crate::constants::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Linux,
    #[serde(rename = "macos")]
    MacOs,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linux => LINUX_PLATFORM,
            Self::MacOs => MACOS_PLATFORM,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    Aarch64,
    X86_64,
}

impl Architecture {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Aarch64 => AARCH64_ARCHITECTURE,
            Self::X86_64 => X86_64_ARCHITECTURE,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Target {
    pub platform: Platform,
    pub architecture: Architecture,
}
