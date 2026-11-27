use super::concurrency_support::{BuilderPause, CONCURRENCY_TIMEOUT};
use super::support::*;
use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn failed_builder_wakes_waiter_and_releases_the_reservation() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = Arc::new(fixture.router());
    let admission = authority.durable();
    let pause = Arc::new(BuilderPause::default());
    let owner_router = Arc::clone(&router);
    let owner_admission = admission.clone();
    let owner_pause = Arc::clone(&pause);
    let owner = std::thread::spawn(move || {
        owner_router.install_or_replay(owner_admission, |_| {
            owner_pause.block();
            Err(CodexRouterError::Runtime)
        })
    });
    pause.wait_until_entered();

    let follower_router = Arc::clone(&router);
    let follower_runtime = Arc::clone(&authority.runtime);
    let follower_calls = Arc::new(AtomicUsize::new(usize::default()));
    let calls = Arc::clone(&follower_calls);
    let follower = std::thread::spawn(move || {
        follower_router.install_or_replay(admission, |_| {
            calls.fetch_add(SINGLE_ENTRY_COUNT, Ordering::SeqCst);
            Ok(follower_runtime)
        })
    });
    let waiter_ready = router.wait_for_build_waiters_for_test(
        &authority.identity,
        SINGLE_ENTRY_COUNT,
        CONCURRENCY_TIMEOUT,
    );
    pause.release();

    assert!(waiter_ready.unwrap());
    assert_eq!(
        owner.join().unwrap().err().unwrap(),
        CodexRouterError::Runtime
    );
    assert_eq!(
        follower.join().unwrap().err().unwrap(),
        CodexRouterError::Runtime
    );
    assert_eq!(follower_calls.load(Ordering::SeqCst), usize::default());
    assert_eq!(router.len().unwrap(), usize::default());
    assert!(authority.install(router.as_ref()).is_ok());
}

#[test]
fn panicking_builder_wakes_waiter_and_does_not_strand_building() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = Arc::new(fixture.router());
    let admission = authority.durable();
    let pause = Arc::new(BuilderPause::default());
    let owner_router = Arc::clone(&router);
    let owner_admission = admission.clone();
    let owner_pause = Arc::clone(&pause);
    let owner = std::thread::spawn(move || {
        owner_router.install_or_replay(owner_admission, |_| {
            owner_pause.block();
            std::panic::panic_any(())
        })
    });
    pause.wait_until_entered();

    let follower_router = Arc::clone(&router);
    let follower_runtime = Arc::clone(&authority.runtime);
    let follower = std::thread::spawn(move || {
        follower_router.install_or_replay(admission, |_| Ok(follower_runtime))
    });
    let waiter_ready = router.wait_for_build_waiters_for_test(
        &authority.identity,
        SINGLE_ENTRY_COUNT,
        CONCURRENCY_TIMEOUT,
    );
    pause.release();

    assert!(waiter_ready.unwrap());
    assert!(owner.join().is_err());
    assert_eq!(
        follower.join().unwrap().err().unwrap(),
        CodexRouterError::Lock
    );
    assert_eq!(router.len().unwrap(), usize::default());
    assert!(authority.install(router.as_ref()).is_ok());
}

#[test]
fn building_reservation_counts_toward_router_capacity() {
    let fixture = Fixture::new();
    let building = fixture.authority();
    let rejected = fixture.authority();
    let router = Arc::new(fixture.router_with_capacity(SINGLE_ENTRY_COUNT));
    let pause = Arc::new(BuilderPause::default());
    let owner_router = Arc::clone(&router);
    let owner_runtime = Arc::clone(&building.runtime);
    let owner_pause = Arc::clone(&pause);
    let owner = std::thread::spawn(move || {
        owner_router.install_or_replay(building.durable(), |_| {
            owner_pause.block();
            Ok(owner_runtime)
        })
    });
    pause.wait_until_entered();

    let rejected_calls = AtomicUsize::new(usize::default());
    let result = router.install_or_replay(rejected.durable(), |_| {
        rejected_calls.fetch_add(SINGLE_ENTRY_COUNT, Ordering::SeqCst);
        Ok(Arc::clone(&rejected.runtime))
    });
    assert_eq!(result.err().unwrap(), CodexRouterError::Capacity);
    assert_eq!(rejected_calls.load(Ordering::SeqCst), usize::default());
    assert_eq!(router.len().unwrap(), SINGLE_ENTRY_COUNT);

    pause.release();
    assert!(owner.join().unwrap().is_ok());
}
