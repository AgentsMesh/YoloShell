impl WorkerState {
    pub(crate) fn publish_current_snapshot(&self) -> Result<TerminalSnapshot> {
        let _publication = lock_recover(&self.snapshot_publication);
        self.namespace.verify()?;
        let snapshot = {
            let journal = lock_recover(&self.journal);
            let dimensions = lock_recover(&self.dimensions);
            let terminal = lock_recover(&self.terminal);
            // The prefix describes the same screen and sequence as the
            // contents below. Reading it through `presentation()` released the
            // terminal lock first, so a concurrently committed OSC title could
            // be sequenced into the journal yet missing from this snapshot.
            let mut presentation = terminal.callbacks().presentation().clone();
            if let Some(title) = lock_recover(&self.agent_title).as_ref() {
                presentation.title = Some(title.clone());
            }
            let title_prefix =
                yolo_terminal_metadata::MetadataCallbacks::new(presentation).snapshot_prefix();
            TerminalSnapshot {
                runtime_epoch: journal.runtime_epoch(),
                barrier_sequence: journal.last_sequence(),
                cols: dimensions.0,
                rows: dimensions.1,
                data: [title_prefix, terminal.screen().state_formatted()].concat(),
            }
        };
        #[cfg(test)]
        self.control_test_hooks
            .snapshot_before_publication
            .pause_if_armed();
        crate::journal::write_snapshot_retained(
            self.namespace
                .file(&self.paths.snapshot(self.terminal_id()))?,
            &snapshot,
        )?;
        self.namespace.verify()?;
        Ok(snapshot)
    }
}

impl WorkerState {
    /// Visible-screen text for `GET_TEXT`. This is a bounded presentation read:
    /// it carries no lifecycle or control authority.
    pub(crate) fn current_text_admitted(
        &self,
        _permit: &ControllerMutationPermit,
    ) -> Result<TerminalTextResult> {
        let (runtime_epoch, sequence) = {
            let journal = lock_recover(&self.journal);
            (journal.runtime_epoch(), journal.last_sequence())
        };
        let (cols, rows) = *lock_recover(&self.dimensions);
        let text = lock_recover(&self.terminal).screen().contents();
        let (text, truncated) = bound_text(text);
        Ok(TerminalTextResult {
            runtime_epoch,
            sequence,
            cols,
            rows,
            text,
            truncated,
        })
    }
}

fn bound_text(mut text: String) -> (String, bool) {
    if text.len() <= yolo_terminal_model::MAX_TERMINAL_TEXT_BYTES {
        return (text, false);
    }
    let mut end = yolo_terminal_model::MAX_TERMINAL_TEXT_BYTES;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    (text, true)
}
