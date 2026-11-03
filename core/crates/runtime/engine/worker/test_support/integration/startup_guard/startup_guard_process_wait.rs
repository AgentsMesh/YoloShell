pub(in crate::worker::startup::startup_guard) fn wait_for_descendant(marker: &Path) -> ProcessIdentity {
    let deadline = Instant::now() + TEST_READY_TIMEOUT;
    loop {
        if let Ok(value) = std::fs::read_to_string(marker)
            && let Ok(pid) = value.parse()
            && let Ok(identity) = ProcessIdentity::capture(pid)
        {
            return identity;
        }
        assert!(
            Instant::now() < deadline,
            "{TEST_DESCENDANT_TIMEOUT_DETAIL}"
        );
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

pub(in crate::worker::startup::startup_guard) fn wait_for_tracked_identity(
    guard: &StartupChildGuard,
    identity: ProcessIdentity,
) {
    let deadline = Instant::now() + TEST_READY_TIMEOUT;
    while !crate::worker::startup::startup_scope::contains(guard.tree.as_ref().unwrap(), identity) {
        assert!(Instant::now() < deadline, "{TEST_TRACKING_TIMEOUT_DETAIL}");
        std::thread::sleep(TEST_POLL_INTERVAL);
    }
}

pub(super) fn pty_size() -> PtySize {
    PtySize {
        rows: TEST_ROWS,
        cols: TEST_COLUMNS,
        pixel_width: u16::default(),
        pixel_height: u16::default(),
    }
}
