use super::recovery_event_support::{ALL_EVENTS_COUNT, SESSION_START_COUNT};
use super::recovery_test_support::RecoveryFixture;
use crate::agent::AgentState;

#[test]
fn publication_begin_crash_reenters_without_a_second_generation_bump() {
    let fixture = RecoveryFixture::new(ALL_EVENTS_COUNT);
    let _guards = fixture.gone_guards();
    let events = fixture.recovery_events();
    let (started, _) = fixture
        .base
        .store
        .apply_recovery_in_namespace(
            &fixture.base.lease,
            fixture.authority.authentication(),
            &fixture.authority,
            &events[..SESSION_START_COUNT],
        )
        .unwrap();
    let (started, stale) = fixture
        .base
        .store
        .publish_binding_state_in_namespace(
            &fixture.base.lease,
            fixture.binding_store.path(),
            started.authentication(),
            &started,
            false,
            |_| Ok(()),
        )
        .unwrap();
    let (current, _) = fixture
        .base
        .store
        .apply_recovery_in_namespace(
            &fixture.base.lease,
            started.authentication(),
            &started,
            &events[SESSION_START_COUNT..],
        )
        .unwrap();
    let (current, _) = fixture
        .base
        .store
        .publish_binding_state_in_namespace(
            &fixture.base.lease,
            fixture.binding_store.path(),
            current.authentication(),
            &current,
            false,
            |_| Ok(()),
        )
        .unwrap();
    let before = fixture
        .base
        .store
        .publication_generation_in_namespace(&fixture.base.lease)
        .unwrap();
    let pending = fixture
        .base
        .store
        .begin_binding_publication_in_namespace(
            &fixture.base.lease,
            current.authentication(),
            &current,
        )
        .unwrap();
    let after_begin = fixture
        .base
        .store
        .publication_generation_in_namespace(&fixture.base.lease)
        .unwrap();
    assert_eq!(
        after_begin,
        before
            .checked_add(super::super::constants::REVISION_INCREMENT)
            .unwrap()
    );
    assert!(pending.binding_revision.is_none());
    assert!(!pending.matches_binding(&stale));

    let recovered = fixture.load().unwrap().unwrap();
    assert_eq!(recovered.state, AgentState::WaitingForInput);
    assert_eq!(recovered.cwd, fixture.final_cwd.to_string_lossy());
    assert_eq!(
        fixture
            .base
            .store
            .publication_generation_in_namespace(&fixture.base.lease)
            .unwrap(),
        after_begin
    );
    assert_eq!(fixture.load().unwrap(), Some(recovered));
}
