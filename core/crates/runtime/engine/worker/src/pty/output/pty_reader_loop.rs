pub(crate) fn reader_loop(
    mut reader: Box<dyn Read + Send>,
    state: Arc<WorkerState>,
) -> Result<ReaderLoopOutcome> {
    let mut buffer = vec![0_u8; PTY_READ_BUFFER_BYTES];
    let mut output_observed = false;
    let mut staged_chunks = 0_usize;
    let eof_publication = loop {
        let pending_count = state.reader_lifecycle.pending_count();
        let pending_bytes = state.reader_lifecycle.pending_payload_bytes();
        let batch_would_exceed = pending_count > 0
            && lock_recover(&state.journal).batch_would_exceed_limit(
                pending_bytes.saturating_add(buffer.len()),
                pending_count.saturating_add(1),
            );
        if pending_count >= PTY_OUTPUT_BATCH_CHUNK_LIMIT
            || batch_would_exceed
            || (staged_chunks == 0 && state.reader_lifecycle.has_pending())
        {
            if deliver_pending_with_retry(&state)? == PendingDelivery::NamespaceLost {
                staged_chunks = 0;
                if !state.begin_output_drain_after_namespace_loss() {
                    continue;
                }
            } else {
                staged_chunks = 0;
            }
        }
        if state.reader_lifecycle.is_cancelled() {
            if state.reader_lifecycle.has_pending() {
                let _ = deliver_pending_with_retry(&state)?;
            }
            break None;
        }
        let output_read = state.reader_lifecycle.begin_output_read()?;
        let append_attempt = if state.reader_lifecycle.drains_after_namespace_loss()
            || state.reader_lifecycle.has_pending()
        {
            None
        } else {
            match state.prepare_output_append(buffer.len()) {
                Ok(attempt) => Some(attempt),
                Err(error) => {
                    drop(output_read);
                    if state.begin_output_drain_after_namespace_loss() {
                        continue;
                    }
                    return Err(error);
                }
            }
        };
        match output_read.read_from(reader.as_mut(), &mut buffer) {
            PtyReadOutcome::Eof(eof) => {
                let publication = eof.await_eof_publication();
                if state.reader_lifecycle.has_pending() {
                    if deliver_pending_with_retry(&state)? == PendingDelivery::NamespaceLost {
                        staged_chunks = 0;
                        let _ = state.begin_output_drain_after_namespace_loss();
                        drop(publication);
                        continue;
                    }
                }
                break Some(publication);
            }
            PtyReadOutcome::Bytes { attempt, length } => {
                if let Some(append_attempt) = append_attempt {
                    attempt.stage(&buffer[..length], append_attempt)?;
                } else if state.reader_lifecycle.drains_after_namespace_loss() {
                    attempt.discard_after_namespace_loss()?;
                } else {
                    attempt.stage_without_attempt(&buffer[..length])?;
                }
                staged_chunks = staged_chunks.saturating_add(1);
                output_observed = true;
            }
            PtyReadOutcome::Failed { error, .. }
                if error.kind() == std::io::ErrorKind::Interrupted =>
            {
                continue;
            }
            #[cfg(unix)]
            PtyReadOutcome::Failed { attempt, error }
                if error.kind() == std::io::ErrorKind::WouldBlock =>
            {
                if state.reader_lifecycle.has_pending()
                    && deliver_pending_with_retry(&state)? == PendingDelivery::NamespaceLost
                {
                    staged_chunks = 0;
                    if state.begin_output_drain_after_namespace_loss() {
                        continue;
                    }
                }
                staged_chunks = 0;
                attempt.mark_quiescent()?;
                let fd = lock_recover(&state.master)
                    .as_ref()
                    .and_then(|master| master.as_raw_fd())
                    .context(PTY_READER_POLL_INVALID_DETAIL)?;
                match wait_for_pty_readiness(
                    fd,
                    &state.reader_lifecycle.cancelled,
                    &state.reader_lifecycle.output_reader_wakeup,
                )? {
                    PtyReadiness::Cancelled => break None,
                    PtyReadiness::Closed | PtyReadiness::Readable | PtyReadiness::Probe => {}
                }
            }
            PtyReadOutcome::Failed { error, .. } => {
                if state.reader_lifecycle.has_pending() {
                    let _ = deliver_pending_with_retry(&state)?;
                }
                return Err(error).context("read PTY output");
            }
        }
    };
    if state.reader_lifecycle.is_cancelled() {
        drop(eof_publication);
        state.reader_lifecycle.mark_read_stopped();
        return Ok(ReaderLoopOutcome::Cancelled);
    }
    let Some(eof_publication) = eof_publication else {
        state.reader_lifecycle.mark_read_stopped();
        return Ok(ReaderLoopOutcome::Cancelled);
    };
    let deadline = Instant::now() + PTY_READER_EXIT_CONFIRM_TIMEOUT;
    while (!state.child_waited.load(Ordering::Acquire)
        || !state.reader_lifecycle.producers_are_gone())
        && eof_publication.is_current()
        && Instant::now() < deadline
    {
        std::thread::sleep(PTY_READER_EXIT_POLL_INTERVAL);
    }
    if !eof_publication.is_current()
        || !state.child_waited.load(Ordering::Acquire)
        || !state.reader_lifecycle.producers_are_gone()
    {
        return Ok(ReaderLoopOutcome::UnexpectedEof { output_observed });
    }
    eof_publication.publish();
    Ok(ReaderLoopOutcome::ChildExited)
}
