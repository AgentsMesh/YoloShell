use crate::constants::INVALID_IDENTITY;
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IdentityError;

impl fmt::Display for IdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(INVALID_IDENTITY)
    }
}

impl std::error::Error for IdentityError {}

macro_rules! identity {
    ($name:ident) => {
        #[derive(
            Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize,
        )]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            pub fn from_uuid(value: Uuid) -> Result<Self, IdentityError> {
                non_nil(value).map(Self)
            }

            pub const fn as_uuid(self) -> Uuid {
                self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl FromStr for $name {
            type Err = IdentityError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                canonical(value).map(Self)
            }
        }

        impl TryFrom<String> for $name {
            type Error = IdentityError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.to_string()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.hyphenated().fmt(formatter)
            }
        }
    };
}

identity!(HubId);
identity!(TerminalId);
identity!(AgentId);
identity!(RuntimeEpoch);
identity!(ExecutionId);
identity!(FileRootId);
identity!(FileRootEpoch);
identity!(WorkspaceId);
identity!(AgentRunId);
identity!(ConnectorId);
identity!(ConnectorGeneration);
identity!(ConnectorInstanceId);

fn non_nil(value: Uuid) -> Result<Uuid, IdentityError> {
    if value.is_nil() {
        Err(IdentityError)
    } else {
        Ok(value)
    }
}

fn canonical(value: &str) -> Result<Uuid, IdentityError> {
    let value = yolo_crypto::serde_uuid::parse_canonical(value).ok_or(IdentityError)?;
    non_nil(value)
}
