use super::*;

const RETAINED_CURSOR_CONFIRM_DETAIL: &str = "retained cursor did not commit expected tail";

impl Journal {
    pub(super) fn confirm_retained_append(
        &mut self,
        attempt: AppendAttempt,
        payload: &[u8],
        expected: &reconcile_retained::ExpectedAppend,
    ) -> Result<OutputChunk, JournalError> {
        self.commit_cursor.verify_retained_current()?;
        #[cfg(any(test, feature = "runtime-component-tests"))]
        super::tests_retained_hooks::run_confirm_hook()?;
        let committed_bytes = self.commit_cursor.record.committed_bytes;
        #[cfg(any(test, feature = "runtime-component-tests"))]
        let committed_bytes =
            super::tests_retained_hooks::take_confirm_cursor_override().unwrap_or(committed_bytes);
        if committed_bytes != expected.end {
            return Err(anyhow::anyhow!(RETAINED_CURSOR_CONFIRM_DETAIL).into());
        }
        let identity = self.verify_exact_retained_tail(expected)?;
        let state = scan::scan_committed(&mut self.file, expected.end, self.runtime_epoch)?;
        #[cfg(any(test, feature = "runtime-component-tests"))]
        let mut state = state;
        self.entry
            .verify_retained_unchanged(identity, self.max_disk_bytes)?;
        self.file.seek(SeekFrom::End(0))?;
        #[cfg(any(test, feature = "runtime-component-tests"))]
        if let Some(sequence) = super::tests_retained_hooks::take_confirm_sequence_override() {
            state.last_sequence = sequence;
        }
        if state.last_sequence != expected.sequence {
            return Err(JournalError::Corrupt {
                sequence: state.last_sequence,
            });
        }
        self.identity = identity;
        self.records = state.records;
        self.retained_from_sequence = state.retained_from;
        self.last_sequence = state.last_sequence;
        self.disk_bytes = expected.end;
        self.write_healthy = true;
        Ok(OutputChunk {
            runtime_epoch: attempt.runtime_epoch,
            sequence: expected.sequence,
            data: payload.to_vec(),
        })
    }
}
