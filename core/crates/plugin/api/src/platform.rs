use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Unix,
    Macos,
    Linux,
}

impl Platform {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unix => "unix",
            Self::Macos => "macos",
            Self::Linux => "linux",
        }
    }

    pub fn runs_on(self, host: Self) -> bool {
        self == host
            || matches!(
                (self, host),
                (Self::Unix, Self::Linux) | (Self::Unix, Self::Macos)
            )
    }

    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            Self::Macos
        } else {
            Self::Linux
        }
    }
}
