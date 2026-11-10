use super::concurrency_support::{BuilderPause, CONCURRENCY_TIMEOUT};
use super::support::*;
use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn blocked_concurrent_exact_inserts_build_once_and_replay_one_entry() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = Arc::new(fixture.router());
    let admission = authority.durable();
    let pause = Arc::new(BuilderPause::default());
    let builder_calls = Arc::new(AtomicUsize::new(usize::default()));
    let owner = spawn_insert(
        Arc::clone(&router),
        admission.clone(),
        Arc::clone(&authority.runtime),
        Arc::clone(&pause),
        Arc::clone(&builder_calls),
    );
    pause.wait_until_entered();

    let follower_count = CONCURRENT_INSERTS - SINGLE_ENTRY_COUNT;
    let mut followers = Vec::with_capacity(follower_count);
    for _ in usize::default()..follower_count {
        followers.push(spawn_insert(
            Arc::clone(&router),
            admission.clone(),
            Arc::clone(&authority.runtime),
            Arc::clone(&pause),
            Arc::clone(&builder_calls),
        ));
    }
    let all_waiting = router.wait_for_build_waiters_for_test(
        &authority.identity,
        follower_count,
        CONCURRENCY_TIMEOUT,
    );
    pause.release();

    let mut outcomes = vec![owner.join().unwrap().unwrap()];
    outcomes.extend(
        followers
            .into_iter()
            .map(|handle| handle.join().unwrap().unwrap()),
    );
    assert!(all_waiting.unwrap());
    assert_eq!(builder_calls.load(Ordering::SeqCst), SINGLE_ENTRY_COUNT);
    assert_eq!(router.len().unwrap(), SINGLE_ENTRY_COUNT);
    assert_eq!(
        outcomes.iter().filter(|lease| !lease.is_replay()).count(),
        SINGLE_ENTRY_COUNT
    );
    let first = outcomes.first().unwrap().entry();
    assert!(
        outcomes
            .iter()
            .all(|lease| Arc::ptr_eq(first, lease.entry()))
    );
}

#[test]
fn blocked_builder_does_not_block_existing_route_operations() {
    let fixture = Fixture::new();
    let existing = fixture.authority();
    let building = fixture.authority();
    let router = Arc::new(fixture.router());
    existing.install(router.as_ref()).unwrap().settle().unwrap();
    let pause = Arc::new(BuilderPause::default());
    let build = spawn_insert(
        Arc::clone(&router),
        building.durable(),
        Arc::clone(&building.runtime),
        Arc::clone(&pause),
        Arc::new(AtomicUsize::new(usize::default())),
    );
    pause.wait_until_entered();

    let probe_router = Arc::clone(&router);
    let identity = existing.identity.clone();
    let runtime = Arc::clone(&existing.runtime);
    let (probe_sender, probe_receiver) = std::sync::mpsc::sync_channel(SINGLE_ENTRY_COUNT);
    let probe = std::thread::spawn(move || {
        let entry = probe_router.resolve(&identity).unwrap();
        let lease = probe_router.lease_committed(&identity).unwrap();
        let retired = probe_router.retire(&entry).unwrap();
        probe_sender
            .send(
                Arc::ptr_eq(entry.runtime(), &runtime)
                    && Arc::ptr_eq(lease.entry(), &entry)
                    && retired == CodexRouteRetirement::Deferred,
            )
            .unwrap();
    });
    let completed_while_blocked = probe_receiver.recv_timeout(CONCURRENCY_TIMEOUT);
    pause.release();
    let built = build.join().unwrap().unwrap();
    probe.join().unwrap();

    assert!(completed_while_blocked.unwrap());
    assert!(!built.is_replay());
}

#[test]
fn pending_retirement_keeps_the_route_after_existing_leases_settle() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = fixture.router();
    let inserted = authority.install(&router).unwrap();
    let entry = Arc::clone(inserted.entry());
    inserted.settle().unwrap();
    let event = router.lease_committed(&authority.identity).unwrap();

    assert!(matches!(
        router.begin_retirement(&entry).unwrap(),
        CodexRetirementReadiness::Pending
    ));
    event.settle().unwrap();
    assert_eq!(router.len().unwrap(), SINGLE_ENTRY_COUNT);
    ready_retirement(router.begin_retirement(&entry))
        .commit()
        .unwrap();
    assert_eq!(router.len().unwrap(), usize::default());
}

#[test]
fn external_work_with_a_retirement_permit_does_not_hold_the_router_lock() {
    let fixture = Fixture::new();
    let retiring = fixture.authority();
    let healthy = fixture.authority();
    let router = Arc::new(fixture.router());
    let inserted = retiring.install(router.as_ref()).unwrap();
    let entry = Arc::clone(inserted.entry());
    inserted.settle().unwrap();
    let permit = ready_retirement(router.begin_retirement(&entry));
    let worker_router = Arc::clone(&router);
    let worker = std::thread::spawn(move || healthy.install(worker_router.as_ref()));

    let installed = worker.join().unwrap().unwrap();
    assert!(!installed.is_replay());
    drop(permit);
    assert_eq!(router.len().unwrap(), DISTINCT_ENTRY_COUNT);
}

#[test]
fn poisoned_router_fails_permit_commit_closed() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = fixture.router();
    let inserted = authority.install(&router).unwrap();
    let entry = Arc::clone(inserted.entry());
    inserted.settle().unwrap();
    let permit = ready_retirement(router.begin_retirement(&entry));
    router.poison_lock_for_test();

    assert_eq!(permit.commit().unwrap_err(), CodexRouterError::Lock);
}

fn spawn_insert(
    router: Arc<CodexSessionRouter>,
    admission: DurableCodexAdmission,
    runtime: Arc<CodexHookRuntime>,
    pause: Arc<BuilderPause>,
    calls: Arc<AtomicUsize>,
) -> std::thread::JoinHandle<Result<CodexRouteLease, CodexRouterError>> {
    std::thread::spawn(move || {
        router.install_or_replay(admission, |_| {
            calls.fetch_add(SINGLE_ENTRY_COUNT, Ordering::SeqCst);
            pause.block();
            Ok(runtime)
        })
    })
}
