use super::*;
use crate::paths::constants::MAX_DURABLE_JSON_BYTES;
use std::time::{Duration, Instant};

const BOUNDED_TIMEOUT: Duration = Duration::from_millis(40);
const RETRY_DELAY: Duration = Duration::from_millis(2);
const TEST_TIMEOUT: Duration = Duration::from_secs(2);
const TARGET_FILE: &str = "retained.json";
const VALUE: u8 = 1;

#[test]
fn retained_shared_lock_has_an_exact_bounded_deadline() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(TARGET_FILE);
    crate::paths::atomic_write_json(&path, &VALUE).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let (_opened, exclusive) = entry
        .open_retained_with_lock(MAX_DURABLE_JSON_BYTES)
        .unwrap();
    let started = Instant::now();
    let result = entry.open_retained_with_shared_lock_until(
        MAX_DURABLE_JSON_BYTES,
        BOUNDED_TIMEOUT,
        RETRY_DELAY,
    );
    let elapsed = started.elapsed();
    assert!(result.is_err());
    assert!(elapsed >= BOUNDED_TIMEOUT);
    assert!(elapsed < TEST_TIMEOUT);
    drop(exclusive);
    entry
        .open_retained_with_shared_lock_until(MAX_DURABLE_JSON_BYTES, BOUNDED_TIMEOUT, RETRY_DELAY)
        .unwrap();
}

#[test]
fn zero_timeout_performs_one_immediate_shared_lock_attempt() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(TARGET_FILE);
    crate::paths::atomic_write_json(&path, &VALUE).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();

    let immediate = entry
        .open_retained_with_shared_lock_until(MAX_DURABLE_JSON_BYTES, Duration::ZERO, RETRY_DELAY)
        .unwrap();
    drop(immediate);
    let (_opened, exclusive) = entry
        .open_retained_with_lock(MAX_DURABLE_JSON_BYTES)
        .unwrap();
    assert!(
        entry
            .open_retained_with_shared_lock_until(
                MAX_DURABLE_JSON_BYTES,
                Duration::ZERO,
                RETRY_DELAY,
            )
            .is_err()
    );
    drop(exclusive);
}

#[test]
fn retained_shared_lock_rejects_success_at_the_deadline() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(TARGET_FILE);
    crate::paths::atomic_write_json(&path, &VALUE).unwrap();
    let entry = PrivateFileEntry::open(&path).unwrap();
    let (_opened, exclusive) = entry
        .open_retained_with_lock(MAX_DURABLE_JSON_BYTES)
        .unwrap();
    PrivateFileEntry::set_retained_lock_retry_hook(move |deadline| {
        while Instant::now() < deadline {
            std::thread::yield_now();
        }
        drop(exclusive);
    });

    assert!(
        entry
            .open_retained_with_shared_lock_until(
                MAX_DURABLE_JSON_BYTES,
                BOUNDED_TIMEOUT,
                RETRY_DELAY,
            )
            .is_err()
    );
    entry
        .open_retained_with_shared_lock_until(MAX_DURABLE_JSON_BYTES, BOUNDED_TIMEOUT, RETRY_DELAY)
        .unwrap();
}
