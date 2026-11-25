use super::*;

const INITIAL: Duration = Duration::from_millis(10);
const MAXIMUM: Duration = Duration::from_millis(40);
const LIMIT: usize = 3;

#[cfg(feature = "yolo-unit-tests")] #[test]
fn accept_backoff_caps_and_resets_after_success() {
    let mut backoff = AcceptBackoff::new(INITIAL, MAXIMUM);
    assert_eq!(backoff.next_delay(), INITIAL);
    assert_eq!(backoff.next_delay(), INITIAL * ACCEPT_BACKOFF_MULTIPLIER);
    assert_eq!(backoff.next_delay(), MAXIMUM);
    assert_eq!(backoff.next_delay(), MAXIMUM);
    backoff.reset();
    assert_eq!(backoff.failures(), 0);
    assert_eq!(backoff.next_delay(), INITIAL);
}

#[cfg(feature = "yolo-unit-tests")] #[test]
fn admission_is_nonblocking_at_limit_and_recovers_on_drop() {
    let clients = Arc::new(Semaphore::new(LIMIT));
    let permits = (0..LIMIT)
        .map(|_| try_admit(&clients).unwrap())
        .collect::<Vec<_>>();
    assert!(try_admit(&clients).is_none());
    drop(permits);
    assert!(try_admit(&clients).is_some());
}
