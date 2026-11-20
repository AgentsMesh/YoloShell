use std::fmt::{Display, Formatter};

#[derive(Debug)]
struct CodexAuthoritySuperseded;

impl Display for CodexAuthoritySuperseded {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(super::super::constants::CODEX_AUTHORITY_SUPERSEDED_ERROR)
    }
}

impl std::error::Error for CodexAuthoritySuperseded {}

pub(in crate::worker::codex_hook) fn is_superseded_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<CodexAuthoritySuperseded>().is_some()
}

pub(in crate::worker::codex_hook) fn superseded_error() -> anyhow::Error {
    anyhow::Error::new(CodexAuthoritySuperseded)
}
