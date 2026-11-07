//! Creation-authorization stdin documents.
//!
//! Operators supply the authority as UUID text (one UUID per authority field);
//! the legacy Cap'n Proto envelope remains accepted for existing automation.
//! Both forms stay on stdin so the capability never appears in argv or
//! diagnostics.

use anyhow::Context as _;
use clap::Args;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::io::{IsTerminal, Read};
#[cfg(test)]
use crate::ctl_args::test_input::TestInput;

const CREATION_STDIN_CONTEXT: &str = "read creation authorization from stdin";
const CREATION_STDIN_TTY_ERROR: &str =
    "refusing to read creation authorization from an interactive terminal";
const CREATION_STDIN_LIMIT_ERROR: &str = "creation authorization exceeds the stdin limit";
const CREATION_STDIN_FORMAT_ERROR: &str =
    "creation authorization needs one UUID per authority field, or the legacy Cap'n Proto document";
const NIL_CREATION_AUTHORITY_ERROR: &str = "creation authorization UUID must not be nil";
const REDACTED_AUTHORITY: &str = "[REDACTED]";
pub(crate) const MAX_CREATION_AUTHORITY_BYTES: u64 = 4_096;
const AUTHORITY_OVERFLOW_READ_BYTES: u64 = MAX_CREATION_AUTHORITY_BYTES + 1;

#[derive(Debug, Args)]
pub(crate) struct CreationAuthorization {
    /// Read creation authorization from stdin as authority UUID text (one UUID
    /// per field) or the legacy Cap'n Proto document.
    #[arg(
        long = yolo_runtime::cli::constants::CREATION_AUTHORIZATION_STDIN_NAME,
        required = true,
        action = clap::ArgAction::SetTrue
    )]
    from_stdin: bool,
    #[cfg(test)]
    #[arg(skip)]
    test_input: Option<TestInput>,
}

impl CreationAuthorization {
    pub(super) fn read<T: CreationAuthorityFields>(self) -> anyhow::Result<T> {
        #[cfg(test)]
        if let Some(encoded) = self.test_input.clone() {
            return self.read_checked(encoded.as_slice(), false);
        }
        let stdin = std::io::stdin();
        let interactive_terminal = stdin.is_terminal();
        self.read_checked(stdin, interactive_terminal)
    }

    pub(super) fn read_checked<T: CreationAuthorityFields>(
        self,
        reader: impl Read,
        interactive_terminal: bool,
    ) -> anyhow::Result<T> {
        anyhow::ensure!(!interactive_terminal, CREATION_STDIN_TTY_ERROR);
        anyhow::ensure!(self.from_stdin, CREATION_STDIN_CONTEXT);
        let mut encoded = Vec::new();
        reader
            .take(AUTHORITY_OVERFLOW_READ_BYTES)
            .read_to_end(&mut encoded)
            .context(CREATION_STDIN_CONTEXT)?;
        anyhow::ensure!(
            encoded.len() as u64 <= MAX_CREATION_AUTHORITY_BYTES,
            CREATION_STDIN_LIMIT_ERROR
        );
        decode_creation_authorization(&encoded).context(CREATION_STDIN_CONTEXT)
    }

    #[cfg(test)]
    pub(super) fn set_test_input(&mut self, encoded: Vec<u8>) {
        self.test_input = Some(TestInput::new(encoded));
    }
}

/// The operator-facing form is authority UUID text; printable input that does
/// not parse as UUIDs is a format error, while binary input is the legacy
/// Cap'n Proto document. Cap'n Proto messages always contain control bytes.
fn decode_creation_authorization<T: CreationAuthorityFields>(
    encoded: &[u8],
) -> anyhow::Result<T> {
    match text_authorization(encoded)? {
        Some(authorization) => Ok(authorization),
        None => Ok(yolo_capnp_codec::decode(encoded)?),
    }
}

fn text_authorization<T: CreationAuthorityFields>(
    encoded: &[u8],
) -> anyhow::Result<Option<T>> {
    let Ok(text) = std::str::from_utf8(encoded) else {
        return Ok(None);
    };
    if !text
        .chars()
        .all(|character| character.is_ascii_graphic() || character.is_ascii_whitespace())
    {
        return Ok(None);
    }
    let mut values = Vec::new();
    for token in text.split_whitespace() {
        values.push(uuid::Uuid::parse_str(token).context(CREATION_STDIN_FORMAT_ERROR)?);
    }
    T::from_uuids(&values)
        .map(Some)
        .context(CREATION_STDIN_FORMAT_ERROR)
}

pub(crate) trait CreationAuthorityFields: DeserializeOwned {
    const UUID_FIELDS: usize;

    fn from_uuids(values: &[uuid::Uuid]) -> Option<Self>;
}

macro_rules! creation_authority {
    ($name:ident { $($field:ident),+ $(,)? }) => {
        #[derive(Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
        #[serde(deny_unknown_fields)]
        pub(crate) struct $name {
            $(pub(crate) $field: uuid::Uuid,)+
        }

        impl $name {
            pub(crate) fn validate(self) -> anyhow::Result<Self> {
                anyhow::ensure!(
                    ![$(self.$field),+].into_iter().any(|value| value.is_nil()),
                    NIL_CREATION_AUTHORITY_ERROR
                );
                Ok(self)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    $(.field(stringify!($field), &REDACTED_AUTHORITY))+
                    .finish()
            }
        }

        impl CreationAuthorityFields for $name {
            const UUID_FIELDS: usize = [$(stringify!($field)),+].len();

            fn from_uuids(values: &[uuid::Uuid]) -> Option<Self> {
                if values.len() != Self::UUID_FIELDS {
                    return None;
                }
                let mut values = values.iter().copied();
                Some(Self {
                    $($field: values.next()?,)+
                })
            }
        }
    };
}

creation_authority!(CreateAuthority { creation_token });
creation_authority!(CleanupAuthority {
    creation_capability,
});
creation_authority!(FinalizeAuthority {
    creation_token,
    creation_capability,
});
