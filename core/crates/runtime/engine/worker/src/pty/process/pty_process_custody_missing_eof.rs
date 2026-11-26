use super::PtyProcessCustody;
use crate::worker::pty_missing_eof::{AbandonedPtyOutput, MissingEofCustody};
use anyhow::Result;

pub(crate) const CUSTODY_NOT_REPORTED_DETAIL: &str =
    "PTY output cannot be abandoned before tracked producers are reported gone";

impl MissingEofCustody for PtyProcessCustody {
    fn producers_are_reported_gone(&self) -> bool {
        matches!(self, Self::GoneReported(_) | Self::MasterClosePending(_))
    }

    fn retire_abandoned_output(&mut self, _proof: AbandonedPtyOutput) -> Result<()> {
        let Self::GoneReported(_) = self else {
            anyhow::bail!(CUSTODY_NOT_REPORTED_DETAIL);
        };
        Ok(())
    }
}
