use crate::journal::Journal;
use anyhow::{Context, Result};
use std::fs::File;
use std::io::{self, Read};
use std::sync::{Arc, Mutex};

pub(crate) const OUTPUT_READ_BUFFER_BYTES: usize = 8 * 1024;
pub(crate) const OUTPUT_PROGRESS_CHUNK_BUDGET: usize = 32;
pub(crate) const OUTPUT_READ_CONTEXT: &str = "read PTY output during process settlement";
pub(crate) const OUTPUT_RETAINED_COMMIT_CONTEXT: &str =
    "commit startup PTY output through retained journal authority";

pub(crate) type SharedJournal = Arc<Mutex<Journal>>;

pub(crate) struct ExclusiveJournalOutput {
    pub(crate) journal: SharedJournal,
    pub(crate) pending: Vec<Vec<u8>>,
}

impl ExclusiveJournalOutput {
    pub(crate) fn new(journal: SharedJournal) -> Self {
        Self {
            journal,
            pending: Vec::new(),
        }
    }

    #[cfg(test)]
    pub(crate) fn progress(&mut self, file: &File) -> Result<super::OutputProgressState> {
        let mut reader = file;
        self.progress_with(&mut reader)
    }

    pub(crate) fn progress_observing_reads<Observed>(
        &mut self,
        file: &File,
        observed: Observed,
    ) -> Result<super::OutputProgressState>
    where
        Observed: FnMut(),
    {
        let mut reader = file;
        self.progress_with_observer(&mut reader, observed)
    }

    #[cfg(test)]
    pub(crate) fn progress_with<Reader: Read>(
        &mut self,
        reader: &mut Reader,
    ) -> Result<super::OutputProgressState> {
        self.progress_with_observer(reader, || {})
    }

    pub(crate) fn progress_with_observer<Reader: Read, Observed: FnMut()>(
        &mut self,
        reader: &mut Reader,
        mut observed: Observed,
    ) -> Result<super::OutputProgressState> {
        let mut buffer = [u8::default(); OUTPUT_READ_BUFFER_BYTES];
        let mut chunks = 0_usize;
        loop {
            if chunks >= OUTPUT_PROGRESS_CHUNK_BUDGET {
                return Ok(super::OutputProgressState::Pending);
            }
            match reader.read(&mut buffer) {
                Ok(0) => {
                    self.commit_pending()?;
                    return Ok(super::OutputProgressState::Complete);
                }
                Ok(length) => {
                    observed();
                    self.pending.push(buffer[..length].to_vec());
                    chunks = chunks.saturating_add(1);
                }
                Err(error) if error.kind() == io::ErrorKind::Interrupted => {}
                Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                    self.commit_pending()?;
                    return Ok(super::OutputProgressState::Quiescent);
                }
                Err(error)
                    if super::super::pty_output_terminal::is_kernel_pty_eof_error(&error) =>
                {
                    self.commit_pending()?;
                    return Ok(super::OutputProgressState::Complete);
                }
                Err(error) => return Err(error).context(OUTPUT_READ_CONTEXT),
            }
        }
    }

    pub(crate) fn commit_pending(&mut self) -> Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        let mut journal = super::super::lock_recover(&self.journal);
        let payloads: Vec<&[u8]> = self.pending.iter().map(Vec::as_slice).collect();
        let base = journal.last_sequence();
        if let Err(error) = journal.append_batch(&payloads) {
            reconcile_batch(&mut journal)
                .context(OUTPUT_RETAINED_COMMIT_CONTEXT)
                .context(error)?;
            let recovered = journal
                .replay_after(base, usize::MAX)
                .context(OUTPUT_RETAINED_COMMIT_CONTEXT)?;
            let batch_committed = recovered.len() == self.pending.len()
                && recovered
                    .iter()
                    .zip(self.pending.iter())
                    .all(|(chunk, payload)| chunk.data == *payload);
            if !batch_committed {
                for payload in &self.pending {
                    let mut attempt = None;
                    super::super::append_journal_output(&mut journal, payload, &mut attempt)
                        .context(OUTPUT_RETAINED_COMMIT_CONTEXT)?;
                }
            }
        }
        drop(journal);
        self.pending.clear();
        Ok(())
    }
}

pub(crate) fn reconcile_batch(journal: &mut Journal) -> Result<()> {
    let mut recoveries = 0_usize;
    loop {
        match journal.reconcile() {
            Ok(()) => return Ok(()),
            Err(error) if recoveries < super::super::JOURNAL_APPEND_RECOVERY_ATTEMPTS => {
                recoveries = recoveries.saturating_add(1);
                let _ = error;
            }
            Err(error) => return Err(error.into()),
        }
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../../integration_tests/pty/output/pty_output_journal_tests.rs"]
mod tests;
