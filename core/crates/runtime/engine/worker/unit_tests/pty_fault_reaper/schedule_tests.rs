use super::*;

const INITIAL_DELAY: Duration = Duration::from_millis(10);
const POLL_LIMIT: Duration = crate::worker::termination_timing::GRACEFUL_TERMINATION_POLL_INTERVAL;
const CURRENT_DELAY: Duration = Duration::from_millis(80);
const MAXIMUM_DELAY: Duration = Duration::from_millis(160);
const PROMPT_BUDGET: Duration = Duration::from_secs(1);
const SHORT_REMAINING_BUDGET: Duration = Duration::from_millis(5);
const AFTER_DEADLINE_OFFSET: Duration = Duration::from_millis(1);

#[test]
fn prompt_retries_are_capped_by_the_protocol_poll_interval() {
    let now = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(now, PROMPT_BUDGET);

    assert_eq!(
        schedule.retry_at(
            CURRENT_DELAY,
            FaultReapActivity::Idle,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            now,
        ),
        (POLL_LIMIT, POLL_LIMIT)
    );
}

#[test]
fn prompt_sleep_never_crosses_the_absolute_deadline() {
    let now = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(now, SHORT_REMAINING_BUDGET);

    assert_eq!(
        schedule.retry_at(
            CURRENT_DELAY,
            FaultReapActivity::Idle,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            now,
        ),
        (SHORT_REMAINING_BUDGET, POLL_LIMIT)
    );
}

#[test]
fn background_no_progress_uses_the_current_backoff() {
    let started = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(started, PROMPT_BUDGET);
    let after_deadline = started + PROMPT_BUDGET + AFTER_DEADLINE_OFFSET;

    assert_eq!(
        schedule.retry_at(
            CURRENT_DELAY,
            FaultReapActivity::Idle,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            after_deadline,
        ),
        (CURRENT_DELAY, MAXIMUM_DELAY)
    );
}

#[test]
fn prompt_authoritative_progress_runs_the_next_bounded_poll_immediately() {
    let now = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(now, PROMPT_BUDGET);

    assert_eq!(
        schedule.retry_at(
            MAXIMUM_DELAY,
            FaultReapActivity::SemanticProgress,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            now,
        ),
        (Duration::ZERO, INITIAL_DELAY)
    );
}

#[test]
fn background_authoritative_progress_restores_the_initial_delay() {
    let started = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(started, PROMPT_BUDGET);
    let after_deadline = started + PROMPT_BUDGET + AFTER_DEADLINE_OFFSET;

    assert_eq!(
        schedule.retry_at(
            MAXIMUM_DELAY,
            FaultReapActivity::SemanticProgress,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            after_deadline,
        ),
        (INITIAL_DELAY, INITIAL_DELAY)
    );
}

#[test]
fn background_backoff_stays_at_its_named_maximum() {
    let now = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(now, Duration::ZERO);

    assert_eq!(
        schedule.retry_at(
            MAXIMUM_DELAY,
            FaultReapActivity::Idle,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            now,
        ),
        (MAXIMUM_DELAY, MAXIMUM_DELAY)
    );
}

#[test]
fn prompt_backoff_debt_cannot_cross_into_background_cleanup() {
    let started = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(started, PROMPT_BUDGET);
    let before_deadline = started + PROMPT_BUDGET - SHORT_REMAINING_BUDGET;
    let deadline = started + PROMPT_BUDGET;
    let (_, prompt_next) = schedule.retry_at(
        MAXIMUM_DELAY,
        FaultReapActivity::Idle,
        INITIAL_DELAY,
        MAXIMUM_DELAY,
        before_deadline,
    );

    assert_eq!(prompt_next, POLL_LIMIT);
    assert_eq!(
        schedule.retry_at(
            prompt_next,
            FaultReapActivity::Idle,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            deadline,
        ),
        (
            POLL_LIMIT,
            POLL_LIMIT.saturating_mul(
                crate::worker::pty_fault_reaper::constants::PTY_FAULT_REAPER_RETRY_MULTIPLIER
            )
        )
    );
}

#[test]
fn split_scan_continues_immediately_without_erasing_backoff_debt() {
    let started = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(started, Duration::ZERO);

    let (wait, retained) = schedule.retry_at(
        CURRENT_DELAY,
        FaultReapActivity::ScanContinuation,
        INITIAL_DELAY,
        MAXIMUM_DELAY,
        started,
    );

    assert_eq!(wait, Duration::ZERO);
    assert_eq!(retained, CURRENT_DELAY);
}

#[test]
fn complete_negative_round_resumes_exponential_backoff_after_split_batches() {
    let started = Instant::now();
    let schedule = FaultReapSchedule::with_timeout(started, Duration::ZERO);
    let (_, retained) = schedule.retry_at(
        CURRENT_DELAY,
        FaultReapActivity::ScanContinuation,
        INITIAL_DELAY,
        MAXIMUM_DELAY,
        started,
    );

    assert_eq!(
        schedule.retry_at(
            retained,
            FaultReapActivity::Idle,
            INITIAL_DELAY,
            MAXIMUM_DELAY,
            started,
        ),
        (CURRENT_DELAY, MAXIMUM_DELAY)
    );
}
