use super::concurrency_support::{BuilderPause, CONCURRENCY_TIMEOUT};
use super::conflict_support::ConflictField;
use super::support::*;
use super::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn distinct_invocation_builders_enter_concurrently() {
    let fixture = Fixture::new();
    let first = fixture.authority();
    let second = fixture.authority();
    let router = Arc::new(fixture.router());
    let pause = Arc::new(BuilderPause::default());
    let first_build = spawn_blocked_build(
        Arc::clone(&router),
        first.durable(),
        Arc::clone(&first.runtime),
        Arc::clone(&pause),
    );
    let second_build = spawn_blocked_build(
        Arc::clone(&router),
        second.durable(),
        Arc::clone(&second.runtime),
        Arc::clone(&pause),
    );

    let both_entered = pause.wait_for_entries(DISTINCT_ENTRY_COUNT);
    pause.release();
    let first_lease = first_build.join().unwrap().unwrap();
    let second_lease = second_build.join().unwrap().unwrap();

    assert!(both_entered);
    assert!(!first_lease.is_replay());
    assert!(!second_lease.is_replay());
}

#[test]
fn conflicting_admission_does_not_wait_for_same_key_builder() {
    let fixture = Fixture::new();
    let authority = fixture.authority();
    let router = Arc::new(fixture.router());
    let pause = Arc::new(BuilderPause::default());
    let owner_router = Arc::clone(&router);
    let owner_runtime = Arc::clone(&authority.runtime);
    let owner_pause = Arc::clone(&pause);
    let owner_admission = authority.durable();
    let owner = std::thread::spawn(move || {
        owner_router.install_or_replay(owner_admission, |_| {
            owner_pause.block();
            Ok(owner_runtime)
        })
    });
    pause.wait_until_entered();

    std::fs::remove_file(authority.storage.path()).unwrap();
    let changed = fixture.changed_record(&authority, ConflictField::Cwd);
    let publication = authority.storage.publish_for_test(&changed).unwrap();
    let conflicting =
        DurableCodexAdmission::from_publication(authority.storage.clone(), publication).unwrap();
    let conflict_router = Arc::clone(&router);
    let builder_calls = Arc::new(AtomicUsize::new(usize::default()));
    let calls = Arc::clone(&builder_calls);
    let (sender, receiver) = std::sync::mpsc::sync_channel(SINGLE_ENTRY_COUNT);
    let conflict = std::thread::spawn(move || {
        let result = conflict_router.install_or_replay(conflicting, |_| {
            calls.fetch_add(SINGLE_ENTRY_COUNT, Ordering::SeqCst);
            unreachable!()
        });
        sender.send(result.err()).unwrap();
    });
    let result_while_blocked = receiver.recv_timeout(CONCURRENCY_TIMEOUT);
    pause.release();
    let owner_result = owner.join().unwrap();
    conflict.join().unwrap();

    assert_eq!(
        result_while_blocked.unwrap().unwrap(),
        CodexRouterError::Conflict
    );
    assert_eq!(builder_calls.load(Ordering::SeqCst), usize::default());
    assert_eq!(owner_result.err().unwrap(), CodexRouterError::Admission);
    assert_eq!(router.len().unwrap(), usize::default());
}

fn spawn_blocked_build(
    router: Arc<CodexSessionRouter>,
    admission: DurableCodexAdmission,
    runtime: Arc<CodexHookRuntime>,
    pause: Arc<BuilderPause>,
) -> std::thread::JoinHandle<Result<CodexRouteLease, CodexRouterError>> {
    std::thread::spawn(move || {
        router.install_or_replay(admission, |_| {
            pause.block();
            Ok(runtime)
        })
    })
}
