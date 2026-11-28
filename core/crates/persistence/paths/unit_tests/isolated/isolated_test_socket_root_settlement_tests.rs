use super::*;

const TEST_SETTLEMENT_WINDOW_SECS: u64 = 1;
const TEST_CHUNK_STEP: usize = SETTLEMENT_FINAL_PASS_CHUNKS;

#[derive(Clone, Copy)]
enum MaximumWorkPhase {
    Discovery,
    Stage(usize),
    Completion,
}

struct MaximumWorkScanner {
    namespaces_remaining: usize,
    phase: MaximumWorkPhase,
    chunks_emitted: usize,
}

impl MaximumWorkScanner {
    fn new() -> Self {
        Self {
            namespaces_remaining: MAX_PRIVATE_DIRECTORY_ENTRIES,
            phase: MaximumWorkPhase::Discovery,
            chunks_emitted: 0,
        }
    }

    fn scan_chunk(&mut self) -> ScanChunk {
        self.chunks_emitted += TEST_CHUNK_STEP;
        if self.namespaces_remaining == 0 {
            return ScanChunk::PassComplete;
        }
        match self.phase {
            MaximumWorkPhase::Discovery => {
                self.phase =
                    MaximumWorkPhase::Stage(SETTLEMENT_STAGE_PROGRESS_CHUNKS_PER_NAMESPACE);
            }
            MaximumWorkPhase::Stage(remaining) if remaining > TEST_CHUNK_STEP => {
                self.phase = MaximumWorkPhase::Stage(remaining - TEST_CHUNK_STEP);
            }
            MaximumWorkPhase::Stage(_) => self.phase = MaximumWorkPhase::Completion,
            MaximumWorkPhase::Completion => {
                self.namespaces_remaining -= TEST_CHUNK_STEP;
                self.phase = MaximumWorkPhase::Discovery;
            }
        }
        ScanChunk::Progress
    }
}

#[test]
fn maximum_modeled_namespace_work_completes_on_the_last_budgeted_chunk() {
    let mut scanner = MaximumWorkScanner::new();
    let deadline = Instant::now() + Duration::from_secs(TEST_SETTLEMENT_WINDOW_SECS);

    run_bounded_settlement(&mut scanner, deadline, |scanner| {
        Ok(scanner.scan_chunk() == ScanChunk::PassComplete)
    })
    .unwrap();

    assert_eq!(scanner.chunks_emitted, SETTLEMENT_CHUNK_LIMIT);
}

#[test]
fn one_additional_progress_chunk_exhausts_the_work_budget() {
    let mut chunks_emitted = 0_usize;
    let deadline = Instant::now() + Duration::from_secs(TEST_SETTLEMENT_WINDOW_SECS);
    let completion_chunk = SETTLEMENT_CHUNK_LIMIT + SETTLEMENT_FINAL_PASS_CHUNKS;

    let error = run_bounded_settlement(&mut chunks_emitted, deadline, |chunks| {
        *chunks += TEST_CHUNK_STEP;
        Ok(*chunks == completion_chunk)
    })
    .unwrap_err();

    assert_eq!(error.to_string(), SETTLEMENT_WORK_EXHAUSTED);
    assert_eq!(chunks_emitted, SETTLEMENT_CHUNK_LIMIT);
}

#[test]
fn elapsed_deadline_is_distinct_and_prevents_scanner_work() {
    let mut chunks_emitted = 0_usize;

    let error = run_bounded_settlement(&mut chunks_emitted, Instant::now(), |chunks| {
        *chunks += 1;
        Ok(false)
    })
    .unwrap_err();

    assert_eq!(error.to_string(), SETTLEMENT_DEADLINE_EXHAUSTED);
    assert_eq!(chunks_emitted, 0);
}

#[test]
fn completed_pass_with_retained_namespaces_waits_before_rescanning() {
    let mut pauses = Vec::new();
    let complete = finish_chunk(ScanChunk::PassComplete, || Ok(false), &mut |duration| {
        pauses.push(duration);
    })
    .unwrap();
    assert!(!complete);
    assert_eq!(
        pauses,
        [Duration::from_millis(SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS)]
    );
}

#[test]
fn empty_completed_pass_needs_no_pause() {
    let mut pauses = Vec::new();
    assert!(
        finish_chunk(ScanChunk::PassComplete, || Ok(true), &mut |duration| {
            pauses.push(duration);
        })
        .unwrap()
    );
    assert!(pauses.is_empty());
}

#[test]
fn busy_chunks_use_the_existing_retry_interval_without_claiming_completion() {
    let mut pauses = Vec::new();
    let complete = finish_chunk(ScanChunk::Busy, || panic!(), &mut |duration| {
        pauses.push(duration);
    })
    .unwrap();
    assert!(!complete);
    assert_eq!(
        pauses,
        [Duration::from_millis(SOCKET_LIFECYCLE_LOCK_RETRY_MILLIS)]
    );
}

#[test]
fn progress_is_not_idle_and_does_not_probe_final_namespace_absence() {
    let mut pauses = Vec::new();
    assert!(
        !finish_chunk(ScanChunk::Progress, || panic!(), &mut |duration| {
            pauses.push(duration);
        })
        .unwrap()
    );
    assert!(pauses.is_empty());
}

#[test]
fn absence_verification_error_cannot_become_waiting_or_success() {
    const VERIFICATION_FAILED: &str = "injected namespace identity change";
    let mut pauses = Vec::new();
    let error = finish_chunk(
        ScanChunk::PassComplete,
        || anyhow::bail!(VERIFICATION_FAILED),
        &mut |duration| pauses.push(duration),
    )
    .unwrap_err();
    assert_eq!(error.to_string(), VERIFICATION_FAILED);
    assert!(pauses.is_empty());
}

#[test]
fn idle_passes_still_consume_the_original_finite_work_budget() {
    let deadline = Instant::now() + Duration::from_secs(TEST_SETTLEMENT_WINDOW_SECS);
    let mut chunks_emitted = 0_usize;
    let mut pauses = 0_usize;
    let error = run_bounded_settlement(&mut chunks_emitted, deadline, |chunks| {
        *chunks += TEST_CHUNK_STEP;
        finish_chunk(ScanChunk::PassComplete, || Ok(false), &mut |_| {
            pauses += TEST_CHUNK_STEP;
        })
    })
    .unwrap_err();
    assert_eq!(error.to_string(), SETTLEMENT_WORK_EXHAUSTED);
    assert_eq!(chunks_emitted, SETTLEMENT_CHUNK_LIMIT);
    assert_eq!(pauses, SETTLEMENT_CHUNK_LIMIT);
}
