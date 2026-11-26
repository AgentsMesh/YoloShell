use super::*;

pub(super) fn reconcile_journal(journal: &mut Journal) -> Result<()> {
    if journal.requires_reconcile() {
        journal
            .reconcile()
            .context(JOURNAL_RECONCILE_FAILURE_CONTEXT)?;
    }
    Ok(())
}

pub(super) fn append_journal_output(
    journal: &mut Journal,
    data: &[u8],
    pending_attempt: &mut Option<AppendAttempt>,
) -> Result<OutputChunk> {
    let mut recoveries = 0_usize;
    let attempt = *pending_attempt.get_or_insert_with(|| journal.append_attempt());
    loop {
        if journal.requires_reconcile() {
            match reconcile_pending_append(journal, attempt, data, &mut recoveries) {
                Ok(Some(chunk)) => {
                    *pending_attempt = None;
                    return Ok(chunk);
                }
                Ok(None) => {}
                Err(error) => {
                    return Err(error).context(JOURNAL_RECONCILE_FAILURE_CONTEXT);
                }
            }
        }
        match journal.append(data) {
            Ok(chunk) => {
                *pending_attempt = None;
                return Ok(chunk);
            }
            Err(_error) if journal.requires_reconcile() => continue,
            Err(error)
                if error.retryable_without_reconcile()
                    && recoveries < JOURNAL_APPEND_RECOVERY_ATTEMPTS =>
            {
                recoveries += 1;
            }
            Err(error) => return Err(error).context(JOURNAL_APPEND_FAILURE_CONTEXT),
        }
    }
}

pub(super) fn reconcile_pending_append(
    journal: &mut Journal,
    attempt: crate::journal::AppendAttempt,
    data: &[u8],
    recoveries: &mut usize,
) -> std::result::Result<Option<OutputChunk>, JournalError> {
    loop {
        match journal.reconcile_append(attempt, data) {
            Err(error)
                if error.retryable_without_reconcile()
                    && *recoveries < JOURNAL_APPEND_RECOVERY_ATTEMPTS =>
            {
                *recoveries += 1;
            }
            result => return result,
        }
    }
}

pub(super) fn compact_journal_for_output(
    journal: &mut Journal,
    barrier_sequence: u64,
    payload_len: usize,
) -> Result<()> {
    let mut recoveries = 0_usize;
    loop {
        match journal.compact_through(barrier_sequence) {
            Ok(()) => return Ok(()),
            Err(error) if journal.requires_reconcile() => {
                journal
                    .reconcile()
                    .context(JOURNAL_RECONCILE_FAILURE_CONTEXT)
                    .context(error)?;
                if !journal.would_exceed_limit(payload_len) {
                    return Ok(());
                }
            }
            Err(error)
                if error.retryable_without_reconcile()
                    && recoveries < JOURNAL_APPEND_RECOVERY_ATTEMPTS =>
            {
                recoveries += 1;
                continue;
            }
            Err(error) => return Err(error).context(JOURNAL_COMPACTION_FAILURE_CONTEXT),
        }
        if recoveries >= JOURNAL_APPEND_RECOVERY_ATTEMPTS {
            return Err(JournalError::DiskLimitExceeded)
                .context(JOURNAL_COMPACTION_FAILURE_CONTEXT);
        }
        recoveries += 1;
    }
}
