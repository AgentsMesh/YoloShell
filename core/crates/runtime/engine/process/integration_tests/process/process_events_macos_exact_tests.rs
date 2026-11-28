use super::MacosProcessEvents;
use crate::process::LiveProcessIdentity;
use crate::process::inventory_test_support::TestProcess;

const FOREIGN_GENERATION_OFFSET: u64 = 1;

#[test]
fn exact_closure_isolation_requires_a_proven_gone_source() {
    let (_source_process, source) = isolated_identity();
    let mut events = MacosProcessEvents::new(source).unwrap();
    events.unresolved_forks.insert(source.lineage_id());

    assert!(!events.exact_closure_isolated(&[]));
    assert!(!events.exact_closure_isolated(&[source]));

    events.registered.clear();
    assert!(!events.exact_closure_isolated(&[]));
}

#[test]
fn omitted_live_source_remains_watched_until_its_generation_exits() {
    let (_root_process, root) = isolated_identity();
    let (mut source, live) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    assert_eq!(events.reconcile([root, live]).unwrap().registered, [live]);
    events.unresolved_forks.insert(live.lineage_id());

    assert!(events.reconcile([root]).unwrap().registered.is_empty());
    assert_eq!(events.registered.get(&live.pid()), Some(&live));
    assert!(!events.exact_closure_isolated(&[root]));

    source.terminate();
    assert!(events.reconcile([root]).unwrap().registered.is_empty());
    assert!(events.has_unresolved_fork());
    assert_eq!(events.registered.get(&live.pid()), None);
    assert!(events.exact_closure_isolated(&[root]));
    assert!(!events.exact_closure_isolated(&[live]));
}

#[test]
fn omitted_live_source_without_fork_debt_remains_watched_and_invalidates_publication() {
    let (_root_process, root) = isolated_identity();
    let (_source_process, live) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    assert_eq!(events.reconcile([root, live]).unwrap().registered, [live]);

    let outcome = events.reconcile([root]).unwrap();

    assert!(outcome.invalidated);
    assert!(outcome.registered.is_empty());
    assert_eq!(events.registered.get(&live.pid()), Some(&live));
    assert!(!events.has_unresolved_fork());
}

#[test]
fn omitted_live_root_without_exit_authority_remains_watched() {
    let (_root_process, root) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();

    let outcome = events.reconcile(std::iter::empty()).unwrap();

    assert!(outcome.invalidated);
    assert_eq!(events.registered.get(&root.pid()), Some(&root));
}

#[test]
fn exact_root_exit_retires_an_omitted_watcher_before_metadata_disappears() {
    let (_root_process, root) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    events.queue_exit_test_read(root);

    let outcome = events.reconcile(std::iter::empty()).unwrap();

    assert!(outcome.invalidated);
    assert!(root.matches());
    assert!(events.exact_root_exited(root));
    assert_eq!(events.registered.get(&root.pid()), None);
}

#[test]
fn external_root_exit_proof_retires_only_the_exact_generation() {
    let (_root_process, root) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    let foreign = root.with_generation(root.unique_id().wrapping_add(FOREIGN_GENERATION_OFFSET));

    events.confirm_root_exit(foreign);
    assert!(events.reconcile(std::iter::empty()).unwrap().invalidated);
    assert_eq!(events.registered.get(&root.pid()), Some(&root));

    events.confirm_root_exit(root);
    events.reconcile(std::iter::empty()).unwrap();
    assert_eq!(events.registered.get(&root.pid()), None);
}

#[test]
fn external_root_exit_proof_preserves_unresolved_fork_debt() {
    let (_root_process, root) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    events.unresolved_forks.insert(root.lineage_id());

    events.confirm_root_exit(root);

    assert!(events.has_unresolved_fork());
    events.reconcile(std::iter::empty()).unwrap();
    assert!(events.has_unresolved_fork());
    assert!(events.exact_closure_isolated(&[]));
}

#[test]
fn gone_proof_drains_a_final_fork_exit_before_unregistering_the_watcher() {
    let (_root_process, root) = isolated_identity();
    let (mut source, live) = isolated_identity();
    let mut events = MacosProcessEvents::new(root).unwrap();
    assert_eq!(events.reconcile([root, live]).unwrap().registered, [live]);
    source.terminate();
    events.queue_before_unregister_fork_exit_test_read(live);

    let outcome = events.reconcile([root]).unwrap();

    assert!(outcome.invalidated);
    assert_eq!(events.registered.get(&live.pid()), None);
    assert_eq!(
        events.unresolved_fork_lineages_for_test(),
        [live.lineage_id()]
    );
    assert!(events.exact_closure_isolated(&[root]));
}

#[test]
fn coalesced_fork_exit_requires_reconciliation_before_zombie_isolated() {
    let (_source_process, source) = isolated_identity();
    let mut events = MacosProcessEvents::new(source).unwrap();
    events.queue_fork_exit_test_read(source);

    assert!(events.changed().unwrap());
    assert!(events.has_unresolved_fork());
    assert!(!events.exact_closure_isolated(&[]));
    assert_eq!(events.registered.get(&source.pid()), Some(&source));

    assert!(source.matches());
    assert!(events.reconcile([source]).unwrap().registered.is_empty());
    assert!(events.has_unresolved_fork());
    assert_eq!(events.registered.get(&source.pid()), None);
    assert!(events.exact_closure_isolated(&[]));
}

fn isolated_identity() -> (TestProcess, LiveProcessIdentity) {
    let process = TestProcess::spawn();
    let identity = LiveProcessIdentity::capture(process.identity)
        .unwrap()
        .unwrap();
    (process, identity)
}
