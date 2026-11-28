#[test]
fn successful_kernel_attempt_at_the_deadline_is_rejected() {
    let fixture = Fixture::new();
    let holder = fixture.kernel_holder();
    PrivateFileLock::set_wait_retry_hook(move |deadline| {
        while Instant::now() < deadline {
            std::thread::yield_now();
        }
        drop(holder);
    });

    assert!(wait_lock(&fixture.path, BOUNDED_TIMEOUT).is_err());
    assert!(
        PrivateFileLock::try_acquire(&fixture.path, LOCK_MAX_BYTES)
            .unwrap()
            .is_some()
    );
}
