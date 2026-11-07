use clap::Args;
use serde::{Deserialize, Serialize};
use std::io::{IsTerminal, Read};
#[cfg(test)]
use yolo_runtime::cli::constants::WRITER_AUTHORIZATION_STDIN_ARGUMENT;
use yolo_runtime::cli::constants::WRITER_AUTHORIZATION_STDIN_NAME;
use yolo_runtime::model::WriterLeaseGrant;
use yolo_runtime::worker::RuntimeRef;

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-test-support"))] #[path = "../test_support/ctl_args_input.rs"]
pub(crate) mod test_input;
#[cfg(test)]
use test_input::TestInput;

const AUTHORIZATION_STDIN_CONTEXT: &str = "read writer authorization from stdin";
const AUTHORIZATION_STDIN_TTY_ERROR: &str =
    "refusing to read writer authorization from an interactive terminal";
const WRITER_AUTHORIZATION_LIMIT_ERROR: &str = "writer authorization exceeds the stdin limit";
const NIL_WRITER_RUNTIME_ERROR: &str = "writer authorization runtime UUIDs must not be nil";
pub(super) const MAX_WRITER_AUTHORIZATION_BYTES: u64 = 4_096;
const WRITER_AUTHORIZATION_OVERFLOW_BYTES: u64 = MAX_WRITER_AUTHORIZATION_BYTES + 1;

#[cfg(test)]
pub(super) fn writer_authorization_cli_argument() -> String {
    WRITER_AUTHORIZATION_STDIN_ARGUMENT.to_owned()
}

#[derive(Debug, Args)]
pub(super) struct WriterAuthorization {
    #[arg(
        long = WRITER_AUTHORIZATION_STDIN_NAME,
        required = true,
        action = clap::ArgAction::SetTrue
    )]
    from_stdin: bool,
    #[cfg(test)]
    #[arg(skip)]
    test_input: Option<TestInput>,
}

impl WriterAuthorization {
    pub(super) fn read(self) -> anyhow::Result<RuntimeWriterAuthorization> {
        #[cfg(test)]
        if let Some(encoded) = self.test_input.clone() {
            return self.read_checked(encoded.as_slice(), false);
        }
        let stdin = std::io::stdin();
        let interactive_terminal = stdin.is_terminal();
        self.read_checked(stdin, interactive_terminal)
    }

    pub(super) fn read_checked(
        self,
        reader: impl Read,
        interactive_terminal: bool,
    ) -> anyhow::Result<RuntimeWriterAuthorization> {
        anyhow::ensure!(!interactive_terminal, AUTHORIZATION_STDIN_TTY_ERROR);
        anyhow::ensure!(self.from_stdin, AUTHORIZATION_STDIN_CONTEXT);
        let mut encoded = Vec::new();
        reader
            .take(WRITER_AUTHORIZATION_OVERFLOW_BYTES)
            .read_to_end(&mut encoded)
            .context(AUTHORIZATION_STDIN_CONTEXT)?;
        anyhow::ensure!(
            encoded.len() as u64 <= MAX_WRITER_AUTHORIZATION_BYTES,
            WRITER_AUTHORIZATION_LIMIT_ERROR
        );
        let authorization: RuntimeWriterAuthorization =
            yolo_capnp_codec::decode(&encoded).context(AUTHORIZATION_STDIN_CONTEXT)?;
        authorization.validate()
    }

    #[cfg(test)]
    pub(super) fn set_test_input(&mut self, encoded: Vec<u8>) {
        self.test_input = Some(TestInput::new(encoded));
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RuntimeWriterAuthorization {
    pub(super) target: RuntimeRef,
    pub(super) grant: WriterLeaseGrant,
}

impl RuntimeWriterAuthorization {
    pub(super) fn new(target: RuntimeRef, grant: WriterLeaseGrant) -> Self {
        Self { target, grant }
    }

    fn validate(self) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !self.target.terminal_id.is_nil() && !self.target.runtime_epoch.is_nil(),
            NIL_WRITER_RUNTIME_ERROR
        );
        Ok(self)
    }
}

use anyhow::Context as _;

pub(super) use crate::ctl_creation_authorization::{
    CleanupAuthority, CreateAuthority, CreationAuthorization, FinalizeAuthority,
    MAX_CREATION_AUTHORITY_BYTES,
};
