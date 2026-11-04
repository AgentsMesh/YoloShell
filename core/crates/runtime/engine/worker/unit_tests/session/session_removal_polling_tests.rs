include!("../../test_support/unit/session_removal_polling.rs");

#[tokio::test]
async fn slow_removal_scan_yields_and_never_overlaps_or_catches_up() {
    let (probe, mut started, release) = removal_polling_probe();
    let observed = Arc::clone(&probe);
    let mut monitor = Box::pin(wait_for_session_removal_with_observer(move || {
        observed.observe()
    }));
    assert!(poll_removal_once(monitor.as_mut()).is_pending());
    tokio::time::timeout(REMOVAL_POLLING_TIMEOUT, started.recv())
        .await
        .unwrap()
        .unwrap();

    tokio::time::pause();
    tokio::time::advance(ORPHAN_CHECK_INTERVAL * 4).await;
    assert!(poll_removal_once(monitor.as_mut()).is_pending());
    assert_eq!(probe.calls.load(Ordering::Acquire), 1);
    tokio::time::resume();
    let released_at = tokio::time::Instant::now();
    release.send(()).unwrap();
    let next_scan = std::future::poll_fn(|context| {
        assert!(monitor.as_mut().poll(context).is_pending());
        started.poll_recv(context)
    });
    tokio::time::timeout(REMOVAL_POLLING_TIMEOUT, next_scan)
        .await
        .unwrap()
        .unwrap();
    assert!(released_at.elapsed() >= ORPHAN_CHECK_INTERVAL);
    assert_eq!(probe.calls.load(Ordering::Acquire), 2);
    drop(monitor);
    release.send(()).unwrap();
    tokio::time::timeout(REMOVAL_POLLING_TIMEOUT, async {
        while Arc::strong_count(&probe) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn cancelling_removal_monitor_does_not_start_another_scan() {
    let (probe, mut started, release) = removal_polling_probe();
    let observed = Arc::clone(&probe);
    let mut monitor = Box::pin(wait_for_session_removal_with_observer(move || {
        observed.observe()
    }));
    assert!(poll_removal_once(monitor.as_mut()).is_pending());
    tokio::time::timeout(REMOVAL_POLLING_TIMEOUT, started.recv())
        .await
        .unwrap()
        .unwrap();
    drop(monitor);
    release.send(()).unwrap();
    tokio::time::timeout(REMOVAL_POLLING_TIMEOUT, async {
        while Arc::strong_count(&probe) != 1 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(probe.calls.load(Ordering::Acquire), 1);
}

#[tokio::test]
async fn removal_scan_panics_and_io_errors_retry_without_issuing_termination() {
    let fixture = Fixture::new();
    let state = Arc::clone(&fixture.state);
    let calls = Arc::new(AtomicU64::new(0));
    let observed = Arc::clone(&calls);
    let manifest = state.paths.manifest(state.terminal_id());
    let expected = manifest.clone();
    let monitor = wait_for_session_removal_with_observer(move || {
        assert_eq!(state.termination_reason(), None);
        assert!(!state.reader_lifecycle.is_cancelled());
        match observed.fetch_add(1, Ordering::AcqRel) {
            0 => panic!("{REMOVAL_POLLING_PANIC}"),
            1 => SessionRemovalObservation::Indeterminate(
                std::io::Error::from_raw_os_error(libc::EIO).into(),
            ),
            _ => SessionRemovalObservation::Removed(SessionRemoval::ManifestMissing(
                manifest.clone(),
            )),
        }
    });
    let removal = tokio::time::timeout(REMOVAL_POLLING_TIMEOUT, monitor)
        .await
        .unwrap();
    assert_eq!(removal.path(), expected);
    assert_eq!(calls.load(Ordering::Acquire), 3);
    assert_eq!(fixture.state.termination_reason(), None);
}
