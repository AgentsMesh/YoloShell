use super::MacosProcessEvents;
use crate::process::inventory_test_support::TestProcess;
use crate::process::{LiveProcessIdentity, ProcessIdentity};

const REPLACEMENT_GENERATION_INCREMENT: u64 = 1;

#[test]
fn replaced_candidate_invalidates_publication_without_destroying_the_gate() {
    let current = current_identity();
    let replaced = replaced_generation(current);
    let mut events = MacosProcessEvents::new(current).unwrap();

    let outcome = events.reconcile([replaced]).unwrap();

    assert!(outcome.invalidated);
    assert!(outcome.registered.is_empty());
    assert!(events.queue.is_some());
    assert_eq!(events.registered.get(&current.pid()), Some(&current));
}

#[test]
fn registered_stale_generation_is_retired_without_destroying_the_gate() {
    let live = current_identity();
    let stale = replaced_generation(live);
    let mut events = MacosProcessEvents::new(live).unwrap();
    events.registered.insert(live.pid(), stale);

    let outcome = events.reconcile([stale]).unwrap();

    assert!(outcome.invalidated);
    assert!(outcome.registered.is_empty());
    assert!(events.registered.is_empty());
    assert!(events.queue.is_some());
    assert_eq!(events.reconcile([live]).unwrap().registered, [live]);
}

#[test]
fn candidate_missing_at_kernel_registration_preserves_the_existing_event_gate() {
    let root = current_identity();
    let (_candidate_process, candidate) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    events.fail_next_registration_as_missing(candidate);

    let outcome = events.reconcile([root, candidate]).unwrap();

    assert!(outcome.invalidated);
    assert!(outcome.registered.is_empty());
    assert!(events.queue.is_some());
    assert_eq!(events.registered.get(&root.pid()), Some(&root));
    assert_eq!(events.registered.get(&candidate.pid()), None);
    assert_eq!(
        events.reconcile([root, candidate]).unwrap().registered,
        [candidate]
    );
}

#[test]
fn candidate_retired_after_kernel_registration_drains_and_withdraws_its_watcher() {
    let root = current_identity();
    let (mut candidate_process, candidate) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    events.retire_during_next_registration(candidate, move || candidate_process.terminate());

    let outcome = events.reconcile([root, candidate]).unwrap();

    assert!(outcome.invalidated);
    assert!(outcome.registered.is_empty());
    assert!(events.queue.is_some());
    assert_eq!(events.registered.get(&root.pid()), Some(&root));
    assert_eq!(events.registered.get(&candidate.pid()), None);
    assert!(events.unresolved_forks.contains(&candidate.lineage_id()));
    assert!(events.gone_fork_sources.contains(&candidate.lineage_id()));
    let (_fresh_process, fresh) = isolated_identity();
    assert_eq!(events.reconcile([root, fresh]).unwrap().registered, [fresh]);
}

fn current_identity() -> LiveProcessIdentity {
    let durable = ProcessIdentity::capture(std::process::id()).unwrap();
    LiveProcessIdentity::capture(durable).unwrap().unwrap()
}

fn replaced_generation(identity: LiveProcessIdentity) -> LiveProcessIdentity {
    identity.with_generation(
        identity
            .unique_id()
            .checked_add(REPLACEMENT_GENERATION_INCREMENT)
            .unwrap(),
    )
}

fn isolated_identity() -> (TestProcess, LiveProcessIdentity) {
    let process = TestProcess::spawn();
    let identity = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    (process, identity)
}
