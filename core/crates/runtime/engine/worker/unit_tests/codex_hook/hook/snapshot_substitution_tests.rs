use super::support::*;

#[test]
fn retained_replay_rejects_same_content_snapshot_replacement() {
    let fixture = Fixture::new();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    fixture.submit(event.clone()).unwrap();
    crate::codex_hooks::replace_snapshot_same_content(&fixture.runtime.snapshot_path());

    assert_authority_lost(fixture.runtime.ingest(event, fixture.process.pid));

    assert_authority_lost_view(&fixture.runtime);
}

#[test]
fn empty_reconcile_rejects_same_content_snapshot_replacement() {
    let fixture = Fixture::new();
    fixture.submit(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap();
    crate::codex_hooks::replace_snapshot_same_content(&fixture.runtime.snapshot_path());

    assert!(fixture.runtime.reconcile_pending().is_err());

    assert_authority_lost_view(&fixture.runtime);
}

#[test]
fn older_runtime_replay_rejects_a_newer_unacknowledged_snapshot() {
    let fixture = Fixture::new();
    let start = fixture.start(FIRST_EVENT_SEQUENCE);
    fixture.submit(start.clone()).unwrap();
    let (newer, _) = fixture.restart_runtime();
    let outbox =
        crate::codex_hooks::CodexOutbox::for_identity(newer.outbox_path(), &fixture.identity)
            .unwrap();
    let prompt = outbox
        .enqueue(fixture.prompt(UNASSIGNED_EVENT_SEQUENCE))
        .unwrap();
    newer.ingest(prompt, fixture.process.pid).unwrap();

    assert_authority_lost(fixture.runtime.ingest(start, fixture.process.pid));

    assert_authority_lost_view(&fixture.runtime);
}

#[test]
fn post_commit_snapshot_replacement_never_publishes_authority() {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    crate::codex_hooks::replace_snapshot_after_commit(&fixture.runtime.snapshot_path());

    assert_authority_lost(fixture.runtime.ingest(event, fixture.process.pid));

    assert_authority_lost_view(&fixture.runtime);
}

#[test]
fn exhausted_authority_rejects_same_content_snapshot_replacement() {
    let fixture = Fixture::new();
    super::exhaustion_support::install_exhausted_active_authority(&fixture);
    let (runtime, _) = fixture.restart_runtime();
    crate::codex_hooks::replace_snapshot_same_content(&runtime.snapshot_path());

    assert_authority_lost(runtime.ingest(fixture.start(FIRST_EVENT_SEQUENCE), fixture.process.pid));

    assert_authority_lost_view(&runtime);
}

#[test]
fn startup_rejects_snapshot_replacement_before_initial_publication() {
    let fixture = Fixture::new();
    fixture.submit(fixture.start(FIRST_EVENT_SEQUENCE)).unwrap();
    let path = fixture.runtime.snapshot_path();
    super::super::runtime::set_before_initial_publish_hook(move || {
        crate::codex_hooks::replace_snapshot_same_content(&path);
    });

    assert!(fixture.try_restart_runtime().is_err());
}

#[test]
fn target_replacement_at_publication_boundary_never_sends_active() {
    assert_publication_substitution(crate::codex_hooks::replace_snapshot_before_publication);
}

#[test]
fn lock_replacement_at_publication_boundary_never_sends_active() {
    assert_publication_substitution(crate::codex_hooks::replace_snapshot_lock_before_publication);
}

#[test]
fn lock_unlink_at_publication_boundary_never_sends_active() {
    assert_publication_substitution(crate::codex_hooks::remove_snapshot_lock_before_publication);
}

#[test]
fn lock_overwrite_at_publication_boundary_never_sends_active() {
    assert_publication_substitution(crate::codex_hooks::overwrite_snapshot_lock_before_publication);
}

#[test]
fn directory_replacement_at_publication_boundary_never_sends_active() {
    assert_publication_substitution(
        crate::codex_hooks::replace_snapshot_directory_before_publication,
    );
}

#[test]
fn authority_loss_is_absorbing_across_every_runtime_entry() {
    let fixture = Fixture::new();
    let event = fixture.start(FIRST_EVENT_SEQUENCE);
    fixture.submit(event.clone()).unwrap();
    crate::codex_hooks::replace_snapshot_same_content(&fixture.runtime.snapshot_path());
    assert_authority_lost(fixture.runtime.ingest(event.clone(), fixture.process.pid));

    assert_authority_lost(fixture.runtime.ingest(event, fixture.process.pid));
    let reconcile = fixture.runtime.reconcile_pending().unwrap_err();
    assert!(super::super::authority::is_authority_lost_error(&reconcile));
    let observed =
        crate::agent::hooks::ProcessIdentity::new(fixture.process.pid, fixture.process.start_time);
    assert!(matches!(
        fixture.runtime.observe_process_exit(observed),
        Err(super::super::error::CodexHookRejection::AuthorityLost(_))
    ));
    fixture.runtime.contain_authority_panic();
    assert_authority_lost_view(&fixture.runtime);
}

fn assert_publication_substitution(inject: fn(&std::path::Path)) {
    let fixture = Fixture::new();
    let event = fixture.enqueue(fixture.start(UNASSIGNED_EVENT_SEQUENCE));
    inject(&fixture.runtime.snapshot_path());

    assert_authority_lost(fixture.runtime.ingest(event, fixture.process.pid));

    assert_authority_lost_view(&fixture.runtime);
}

fn assert_authority_lost(
    result: Result<
        super::super::runtime::CodexHookPublication,
        super::super::error::CodexHookRejection,
    >,
) {
    assert!(matches!(
        result,
        Err(super::super::error::CodexHookRejection::AuthorityLost(_))
    ));
}

fn assert_authority_lost_view(runtime: &super::super::runtime::CodexHookRuntime) {
    assert!(!runtime.storage_failed().unwrap());
    assert!(runtime.state().is_err());
    assert_eq!(
        runtime.subscribe_authority().borrow().clone(),
        super::super::publication::CodexAuthorityView::AuthorityLost
    );
}
