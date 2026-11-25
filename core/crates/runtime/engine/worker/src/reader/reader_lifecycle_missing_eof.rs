use super::ReaderLifecycle;
use crate::worker::pty_missing_eof::{MissingEofExpired, OutputAbandonment};

impl ReaderLifecycle {
    pub(crate) fn settle_missing_eof(&self, proof: MissingEofExpired) -> OutputAbandonment {
        let mut progress = self.lock_progress();
        if !progress.pending.is_empty()
            || progress.output_read_in_flight
            || progress.eof.has_authority()
            || progress.eof.is_observed()
            || progress.kernel_eof_probe_identity.is_some()
            || progress.kernel_eof_seal_epoch.is_some()
        {
            return OutputAbandonment::Retry;
        }
        progress.output_abandoned = true;
        self.cancel_locked(&mut progress);
        OutputAbandonment::Abandoned(proof.abandon())
    }
}
