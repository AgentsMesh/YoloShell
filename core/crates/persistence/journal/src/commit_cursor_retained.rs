use super::*;

const RETAINED_CURSOR_CHANGED_DETAIL: &str = "retained commit cursor changed";

impl CommitCursor {
    pub(in crate::journal) fn verify_retained_current(&self) -> Result<(), JournalError> {
        let (record, identity) = read_retained_cursor(&self.entry)?;
        if record != self.record || !identity.same_content_version(self.identity) {
            return Err(anyhow::anyhow!(RETAINED_CURSOR_CHANGED_DETAIL).into());
        }
        Ok(())
    }

    pub(in crate::journal) fn verify_retained_evidence(
        &self,
        expected: CommitCursorEvidence,
    ) -> Result<(), JournalError> {
        let (record, identity) = read_retained_cursor(&self.entry)?;
        self.entry
            .verify_retained_unchanged(identity, RETAINED_CURSOR_BYTES as u64)?;
        if record != expected.record || !identity.unchanged(expected.identity) {
            return Err(anyhow::anyhow!(RETAINED_CURSOR_CHANGED_DETAIL).into());
        }
        Ok(())
    }

    pub(in crate::journal) fn publish_retained(
        &mut self,
        committed_bytes: u64,
        journal_entry: &PrivateFileEntry,
        journal_identity: FileIdentity,
        journal_max_bytes: u64,
    ) -> Result<(), JournalError> {
        let previous_record = self.record;
        let previous_identity = self.identity;
        let _lock = self.entry.lock_retained_parent()?;
        self.verify_retained_current()?;
        verify_journal_fence(journal_entry, journal_identity, journal_max_bytes)?;
        let (mut temporary, record) =
            self.entry
                .stage_retained(RETAINED_CURSOR_BYTES as u64, |file, identity| {
                    super::retained_stage::prepare_cursor_write(
                        file,
                        identity,
                        self.entry.path(),
                        previous_record,
                        committed_bytes,
                    )
                })?;
        run_staging_test_hook()?;
        self.verify_retained_current()?;
        let installed = self.entry.install_retained(
            &mut temporary,
            self.identity,
            RETAINED_CURSOR_BYTES as u64,
            || {
                verify_journal_fence(journal_entry, journal_identity, journal_max_bytes)?;
                Ok(())
            },
        )?;
        self.identity = installed;
        self.record = record;
        let validation = run_post_install_test_hook().and_then(|()| {
            self.verify_retained_publication(journal_entry, journal_identity, journal_max_bytes)
        });
        if let Err(error) = validation {
            self.try_rollback_retained_publication(
                &mut temporary,
                installed,
                previous_identity,
                previous_record,
            );
            return Err(error);
        }
        if let Err(error) = self.sync_retained_parent_only() {
            temporary.preserve();
            return Err(error);
        }
        if let Err(error) =
            self.verify_retained_publication(journal_entry, journal_identity, journal_max_bytes)
        {
            self.try_rollback_retained_publication(
                &mut temporary,
                installed,
                previous_identity,
                previous_record,
            );
            return Err(error);
        }
        temporary.cleanup()?;
        Ok(())
    }

    pub(in crate::journal) fn sync_retained_parent(&self) -> Result<(), JournalError> {
        self.sync_retained_parent_only()?;
        super::retained_recovery::settle_retired_cursor(&self.entry, self.record)
    }

    fn sync_retained_parent_only(&self) -> Result<(), JournalError> {
        run_parent_sync_test_hook()?;
        self.entry.sync_retained_parent()?;
        Ok(())
    }

    fn verify_retained_publication(
        &self,
        journal_entry: &PrivateFileEntry,
        journal_identity: FileIdentity,
        journal_max_bytes: u64,
    ) -> Result<(), JournalError> {
        verify_journal_fence(journal_entry, journal_identity, journal_max_bytes)?;
        self.verify_retained_current()
    }

    fn try_rollback_retained_publication(
        &mut self,
        temporary: &mut crate::paths::private_file::RetainedPrivateFile,
        installed: FileIdentity,
        previous_identity: FileIdentity,
        previous_record: CursorRecord,
    ) {
        let rollback = self.entry.rollback_retained_install(
            temporary,
            installed,
            previous_identity,
            RETAINED_CURSOR_BYTES as u64,
        );
        let target_is_previous = self
            .entry
            .verify_retained_content(previous_identity, RETAINED_CURSOR_BYTES as u64)
            .is_ok();
        if rollback.is_err() && !target_is_previous {
            return;
        }
        self.identity = previous_identity;
        self.record = previous_record;
        let _ = self.sync_retained_parent_only();
    }
}

pub(super) fn read_retained_cursor(
    entry: &PrivateFileEntry,
) -> Result<(CursorRecord, FileIdentity), JournalError> {
    let opened = entry.open_retained(RETAINED_CURSOR_BYTES as u64)?;
    let identity = opened.identity();
    let record = super::slots::read_active_cursor(opened.file(), identity, entry.path())?;
    entry.verify_retained_content(identity, RETAINED_CURSOR_BYTES as u64)?;
    Ok((record, identity))
}

fn verify_journal_fence(
    entry: &PrivateFileEntry,
    identity: FileIdentity,
    max_bytes: u64,
) -> Result<(), JournalError> {
    entry.verify_retained_unchanged(identity, max_bytes)?;
    Ok(())
}

fn run_parent_sync_test_hook() -> Result<(), JournalError> {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    super::super::tests_retained_hooks::run_parent_sync_hook()?;
    Ok(())
}

fn run_staging_test_hook() -> Result<(), JournalError> {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    super::super::tests_retained_hooks::run_staging_hook()?;
    Ok(())
}

fn run_post_install_test_hook() -> Result<(), JournalError> {
    #[cfg(any(test, feature = "runtime-component-tests"))]
    super::super::tests_retained_hooks::run_post_install_hook()?;
    Ok(())
}
