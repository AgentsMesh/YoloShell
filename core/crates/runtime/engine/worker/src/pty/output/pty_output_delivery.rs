pub(crate) fn commit_staged_reader_output(
    state: &WorkerState,
) -> std::result::Result<PendingDelivery, PendingCommitFailure> {
    let mut progress = lock_recover(&state.reader_lifecycle.progress);
    if progress.pending.is_empty() {
        return Ok(PendingDelivery::Delivered);
    }
    let payload_storage: Vec<Vec<u8>> = progress
        .pending
        .iter()
        .map(|pending| pending.data.clone())
        .collect();
    let payloads: Vec<&[u8]> = payload_storage.iter().map(Vec::as_slice).collect();
    let attempt = progress.pending.first().and_then(|pending| pending.attempt);
    // Snapshot readers take journal before terminal; publish the whole projection under that lock.
    let mut journal = lock_recover(&state.journal);
    let append = journal.append_batch(&payloads);
    let chunks = match append {
        Ok(chunks) => chunks,
        Err(error) => recover_failed_append(
            state,
            &mut progress,
            &mut journal,
            error.into(),
            attempt,
            &payloads,
        )?,
    };
    for chunk in &chunks {
        state.apply_terminal_output(&chunk.data, chunk.sequence);
    }
    drop(journal);
    let namespace_lost = state.namespace.prove_ownership_loss().is_some();
    let checkpoint_due = chunks
        .iter()
        .any(|chunk| chunk.sequence.is_multiple_of(PTY_SNAPSHOT_INTERVAL_CHUNKS));
    for chunk in chunks {
        let _ = state.events.send(WorkerEvent::Output(chunk));
    }
    progress.pending.clear();
    drop(progress);
    state.reader_lifecycle.changed.notify_all();
    if namespace_lost {
        return Ok(PendingDelivery::NamespaceLost);
    }
    if checkpoint_due && let Err(error) = state.current_snapshot_internal() {
        tracing::warn!(%error, "failed to checkpoint terminal snapshot");
    }
    match state.namespace.verify() {
        Ok(()) => Ok(PendingDelivery::Delivered),
        Err(error) => {
            if state.namespace.prove_ownership_loss().is_some() {
                tracing::error!(%error, PTY_READER_NAMESPACE_LOST_DETAIL);
                Ok(PendingDelivery::NamespaceLost)
            } else {
                Err(PendingCommitFailure::Ordinary {
                    error: error.context(PTY_READER_NAMESPACE_LOST_DETAIL),
                    attempt: INITIAL_PENDING_RETRY_ATTEMPT,
                })
            }
        }
    }
}

pub(crate) fn recover_failed_append(
    state: &WorkerState,
    progress: &mut reader_lifecycle::ReaderProgress,
    journal: &mut Journal,
    error: anyhow::Error,
    attempt: Option<AppendAttempt>,
    payloads: &[&[u8]],
) -> std::result::Result<Vec<OutputChunk>, PendingCommitFailure> {
    if state.namespace.prove_ownership_loss().is_some() {
        tracing::error!(%error, PTY_READER_NAMESPACE_LOST_DETAIL);
        let Some(attempt) = attempt else {
            let retry = progress
                .pending_commit_retries
                .record_retained_commit_uncertain();
            let error = anyhow::anyhow!(PTY_RETAINED_APPEND_ATTEMPT_MISSING_DETAIL)
                .context(PTY_RETAINED_OUTPUT_FAILURE_CONTEXT);
            return Err(PendingCommitFailure::RetainedCommitUncertain {
                error,
                attempt: retry,
            });
        };
        return match journal.reconcile_append_retained_batch_classified(attempt, payloads) {
            RetainedAppendBatchOutcome::Confirmed(chunks) => Ok(chunks),
            RetainedAppendBatchOutcome::DefinitelyUncommitted(retained_error) => {
                let retry = progress
                    .pending_commit_retries
                    .record_retained_definitely_uncommitted();
                let error =
                    anyhow::Error::new(retained_error).context(PTY_RETAINED_OUTPUT_FAILURE_CONTEXT);
                Err(PendingCommitFailure::RetainedDefinitelyUncommitted {
                    error,
                    attempt: retry,
                })
            }
            RetainedAppendBatchOutcome::CommitUncertain(retained_error) => {
                let retry = progress
                    .pending_commit_retries
                    .record_retained_commit_uncertain();
                let error =
                    anyhow::Error::new(retained_error).context(PTY_RETAINED_OUTPUT_FAILURE_CONTEXT);
                Err(PendingCommitFailure::RetainedCommitUncertain {
                    error,
                    attempt: retry,
                })
            }
        };
    }

    if !journal.requires_reconcile() {
        let retry = progress.pending_commit_retries.record_ordinary_failure();
        return Err(PendingCommitFailure::Ordinary {
            error,
            attempt: retry,
        });
    }
    let recovered = attempt
        .map(|attempt| journal.reconcile_append_batch(attempt, payloads))
        .transpose()
        .map_err(|error| {
            let retry = progress.pending_commit_retries.record_ordinary_failure();
            PendingCommitFailure::Ordinary {
                error: error.into(),
                attempt: retry,
            }
        })?;
    match recovered {
        Some(Some(chunks)) => Ok(chunks),
        Some(None) | None => {
            let retry = progress.pending_commit_retries.record_ordinary_failure();
            Err(PendingCommitFailure::Ordinary {
                error,
                attempt: retry,
            })
        }
    }
}
