use super::*;

const HEALTHY_IDLE_TIMEOUT: Duration = Duration::from_millis(500);
const FIRST_PROGRESS_LEAD: Duration = Duration::from_millis(100);
const SECOND_PROGRESS_LAG: Duration = Duration::from_millis(100);
const OBSERVATION_LAG: Duration = Duration::from_millis(10);
const SYNTHETIC_ACTIVE_WINDOW: Duration = Duration::from_secs(1);
const TEST_INSTANT_OVERFLOW: &str = "turn waiter test instant overflow";

#[test]
fn coalesced_healthy_progress_extends_an_unobserved_waiter() {
    let mut waiter = TurnWaiter::idle(HEALTHY_IDLE_TIMEOUT).unwrap();
    let TurnDeadline::Active(original_deadline) = waiter.deadline else {
        panic!("{TEST_INSTANT_OVERFLOW}");
    };
    let first = original_deadline
        .checked_sub(FIRST_PROGRESS_LEAD)
        .expect(TEST_INSTANT_OVERFLOW);
    let second = original_deadline
        .checked_add(SECOND_PROGRESS_LAG)
        .expect(TEST_INSTANT_OVERFLOW);
    let observed = second
        .checked_add(OBSERVATION_LAG)
        .expect(TEST_INSTANT_OVERFLOW);

    waiter.advance_at(first);
    waiter.advance_at(second);

    assert!(waiter.deadline.remaining(observed).unwrap().is_some());
}

#[test]
fn absolute_deadline_ignores_fifo_progress() {
    let original = Instant::now()
        .checked_add(SYNTHETIC_ACTIVE_WINDOW)
        .expect(TEST_INSTANT_OVERFLOW);
    let mut waiter = TurnWaiter::absolute(original);

    waiter.advance_at(Instant::now());

    assert!(matches!(waiter.deadline, TurnDeadline::Active(actual) if actual == original));
}

#[test]
fn renewal_overflow_is_reported_to_the_waiter() {
    let occurred_at = Instant::now();
    let deadline = occurred_at
        .checked_add(SYNTHETIC_ACTIVE_WINDOW)
        .expect(TEST_INSTANT_OVERFLOW);
    let mut waiter = TurnWaiter {
        deadline: TurnDeadline::Active(deadline),
        identity: Arc::new(()),
        idle_timeout: Some(Duration::MAX),
    };

    waiter.advance_at(occurred_at);

    assert!(waiter.deadline.remaining(occurred_at).is_err());
}
