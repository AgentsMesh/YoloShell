pub(crate) const INPUT_DELIVERY_ERROR_UNAVAILABLE_DETAIL: &str = "input delivery error detail unavailable";

impl WorkerState {
    #[cfg(test)]
    pub(crate) fn commit_output(&self, data: &[u8]) -> Result<OutputChunk> {
        let mut attempt = None;
        let chunk = self.commit_output_with_attempt(data, &mut attempt)?;
        self.namespace.verify()?;
        Ok(chunk)
    }

    pub(crate) fn commit_output_with_attempt(
        &self,
        data: &[u8],
        attempt: &mut Option<AppendAttempt>,
    ) -> Result<OutputChunk> {
        if attempt.is_none() {
            *attempt = Some(self.prepare_output_append(data.len())?);
        }
        self.namespace.verify()?;
        let mut journal = lock_recover(&self.journal);
        let chunk = append_journal_output(&mut journal, data, attempt)?;
        self.apply_terminal_output(data, chunk.sequence);
        Ok(chunk)
    }

    pub(crate) fn prepare_output_append(&self, payload_len: usize) -> Result<AppendAttempt> {
        self.namespace.verify()?;
        let mut journal = lock_recover(&self.journal);
        reconcile_journal(&mut journal)?;
        if journal.would_exceed_limit(payload_len) {
            drop(journal);
            let snapshot = self.current_snapshot_internal()?;
            journal = lock_recover(&self.journal);
            reconcile_journal(&mut journal)?;
            if journal.would_exceed_limit(payload_len) {
                compact_journal_for_output(&mut journal, snapshot.barrier_sequence, payload_len)?;
            }
        }
        Ok(journal.prepare_append_attempt(payload_len)?)
    }

    pub(crate) fn apply_terminal_output(&self, data: &[u8], sequence: u64) {
        let mut terminal = lock_recover(&self.terminal);
        let processed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            #[cfg(test)]
            if self.control_test_hooks.take_terminal_parser_panic() {
                panic!("{}", control_test_hooks::INJECTED_TERMINAL_PARSER_PANIC);
            }
            terminal.process(data);
        }));
        let Err(payload) = processed else { return };
        let detail = supervision::panic_detail(payload);
        let presentation = terminal.callbacks().presentation().clone();
        drop(terminal);
        // Hold dimensions across construction and installation so a
        // concurrent resize cannot persist a new geometry while this
        // replacement is built from the old one. The journal is already held
        // by the committing caller, preserving journal -> dimensions ->
        // terminal order.
        let dimensions = lock_recover(&self.dimensions);
        #[cfg(test)]
        self.control_test_hooks
            .parser_recovery_after_dimensions
            .pause_if_armed();
        let (cols, rows) = *dimensions;
        let mut replacement = new_terminal_parser(cols, rows);
        *replacement.callbacks_mut() = yolo_terminal_metadata::MetadataCallbacks::new(presentation);
        // No snapshot or attachment can observe the baseline before the
        // replacement parser is live.
        self.screen_baseline.store(sequence, Ordering::Release);
        *lock_recover(&self.terminal) = replacement;
        drop(dimensions);
        // Once finalization has started, the attached stream will end with the
        // durable exit. A screen reset would force a reattach that admission
        // intentionally rejects until that exit record exists, so keep the
        // existing subscribers instead.
        if !self.is_finalizing() {
            let _ = self.events.send(WorkerEvent::ScreenReset(OutputGapPayload {
                skipped: SCREEN_RESET_SKIPPED_CHUNKS,
                resync_required: true,
            }));
        }
        terminal_reporting::error_detail(&detail, TERMINAL_PARSER_PANIC_DETAIL);
    }

    pub(crate) fn screen_baseline_sequence(&self) -> u64 {
        self.screen_baseline.load(Ordering::Acquire)
    }

    pub(crate) fn persist_json<T: serde::Serialize>(&self, path: &Path, value: &T) -> Result<()> {
        self.namespace.verify()?;
        self.namespace.write_json(path, value)
    }

    pub(crate) fn persist_runtime_record(&self, record: &RuntimeRecord) -> Result<()> {
        publish_runtime_record(&self.namespace, &self.paths, record)
    }
}

pub(crate) fn input_delivery_unknown(input_id: Uuid, error: Option<io::Error>) -> ErrorBody {
    let suffix = error
        .map(|error| {
            let detail = supervision::contain(|| error.to_string())
                .unwrap_or_else(|| INPUT_DELIVERY_ERROR_UNAVAILABLE_DETAIL.to_owned());
            let _ = supervision::contain(|| drop(error));
            format!(": {detail}")
        })
        .unwrap_or_default();
    ErrorBody {
        code: crate::protocol::error_code::DELIVERY_UNKNOWN.into(),
        message: format!("terminal input delivery is unknown{suffix}"),
        retryable: false,
        details: Some(serde_json::json!({
            (crate::protocol::field::INPUT_ID): input_id
        })),
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-unit-tests"))]
#[cfg(any(feature = "yolo-integration-tests", feature = "yolo-worker-unit-state"))]
#[path = "../../unit_tests/state/state_io_containment_tests.rs"]
mod containment_tests;
