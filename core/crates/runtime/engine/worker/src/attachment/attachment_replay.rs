impl WorkerState {
    pub(crate) fn attach_result_admitted(
        &self,
        permit: &ControllerMutationPermit,
        after_seq: u64,
    ) -> std::result::Result<AttachResult, AttachmentResultError> {
        self.attach_result(after_seq, || self.current_snapshot_admitted(permit))
    }

    #[cfg(test)]
    pub(crate) fn attach_result_internal(
        &self,
        after_seq: u64,
    ) -> std::result::Result<AttachResult, AttachmentResultError> {
        self.attach_result(after_seq, || self.current_snapshot_internal())
    }

    pub(crate) fn attach_result<F>(
        &self,
        after_seq: u64,
        snapshot: F,
    ) -> std::result::Result<AttachResult, AttachmentResultError>
    where
        F: FnOnce() -> Result<TerminalSnapshot>,
    {
        if after_seq == FORCE_SNAPSHOT_AFTER_SEQUENCE {
            return resynchronized_attachment(snapshot()?);
        }
        let requested_next =
            attachment_next_sequence(after_seq).ok_or(AttachmentResultError::CursorExhausted)?;
        let mut journal = lock_recover(&self.journal);
        let last = journal.last_sequence();
        if after_seq > last {
            return Err(AttachmentResultError::CursorAhead);
        }
        let screen_baseline = self.screen_baseline_sequence();
        if screen_baseline != 0 && after_seq <= screen_baseline {
            // The parser could not represent the chunk at the baseline, so a
            // replay from at or before it would poison the client parser.
            drop(journal);
            return resynchronized_attachment(snapshot()?);
        }
        let retained_from = journal.retained_from_sequence();
        let cursor_gap = requested_next < retained_from;
        let chunks = if cursor_gap {
            Vec::new()
        } else {
            journal
                .replay_after(after_seq, WORKER_MAX_REPLAY_BYTES)
                .map_err(|error| AttachmentResultError::Internal(error.into()))?
        };
        let replayed_through = chunks
            .last()
            .map(|chunk| chunk.sequence)
            .unwrap_or(after_seq);
        if cursor_gap || replayed_through < last {
            drop(journal);
            return resynchronized_attachment(snapshot()?);
        }
        let next_sequence =
            attachment_next_sequence(last).ok_or(AttachmentResultError::CursorExhausted)?;
        Ok(AttachResult {
            runtime_epoch: journal.runtime_epoch(),
            snapshot: None,
            chunks,
            next_sequence,
            resynchronized: false,
        })
    }
}
