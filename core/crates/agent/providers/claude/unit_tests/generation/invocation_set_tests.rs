use super::super::{
    AuthorityProcessIdentity, GenerationApplyDisposition, GenerationBegin, SessionStartSource,
};
use super::support::*;
use uuid::Uuid;

const SECOND_PROCESS_PID: u32 = PROCESS_PID + 100;
const SECOND_PROCESS_START: u64 = PROCESS_START + 100;
const INVALID_SEQUENCE_GAP: u64 = 2;

#[test]
fn foreground_a_background_b_foreground_a_reactivates_without_a_new_hook_event() {
    let fixture = Fixture::new();
    let pair = parked_pair(&fixture);
    let first = pair.first;
    let first_authentication = pair.first_authentication;
    let first_event = pair.first_event;
    let before = fixture
        .store
        .publication_generation_in_namespace(&fixture.lease)
        .unwrap();
    let (snapshot, disposition) = fixture
        .store
        .preview_invocation_in_namespace(
            &fixture.lease,
            first_authentication,
            first.last_event_sequence,
            &first_event,
        )
        .unwrap();
    assert_eq!(snapshot, first);
    assert_eq!(disposition, GenerationApplyDisposition::Replay);
    let (reactivated, applied, changed) = fixture
        .store
        .transition_foreground_in_namespace(
            &fixture.lease,
            first_authentication,
            &first,
            first.last_event_sequence,
            &first_event,
            first.transcript_identity,
        )
        .unwrap();
    assert_eq!(applied, GenerationApplyDisposition::Replay);
    assert!(changed);
    assert!(reactivated.binding_revision.is_none());
    assert_eq!(
        fixture
            .store
            .publication_generation_in_namespace(&fixture.lease)
            .unwrap(),
        before.checked_add(1).unwrap()
    );
    let committed = fixture
        .store
        .commit_binding_publication_in_namespace(&fixture.lease, first_authentication, &reactivated)
        .unwrap();
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.lease).unwrap(),
        Some(committed)
    );
    assert!(
        fixture
            .store
            .apply_in_namespace(&fixture.lease, pair.second_authentication, &first_event)
            .is_err()
    );
}

#[test]
fn invalid_parked_event_cannot_switch_the_active_invocation() {
    let fixture = Fixture::new();
    let pair = parked_pair(&fixture);
    let first = pair.first;
    let first_authentication = pair.first_authentication;
    let first_event = pair.first_event;
    let active_before = fixture.store.load_in_namespace(&fixture.lease).unwrap();
    let generation_before = fixture
        .store
        .publication_generation_in_namespace(&fixture.lease)
        .unwrap();
    let invalid_sequence = first
        .last_event_sequence
        .checked_add(INVALID_SEQUENCE_GAP)
        .unwrap();
    let (unchanged, disposition, reactivated) = fixture
        .store
        .transition_foreground_in_namespace(
            &fixture.lease,
            first_authentication,
            &first,
            invalid_sequence,
            &first_event,
            first.transcript_identity,
        )
        .unwrap();
    assert_eq!(unchanged, first);
    assert_eq!(disposition, GenerationApplyDisposition::Invalidated);
    assert!(!reactivated);
    assert_eq!(
        fixture.store.load_in_namespace(&fixture.lease).unwrap(),
        active_before
    );
    assert_eq!(
        fixture
            .store
            .publication_generation_in_namespace(&fixture.lease)
            .unwrap(),
        generation_before
    );
}

pub(super) struct ParkedPair {
    pub first: super::super::ClaudeGenerationAuthority,
    pub first_authentication: super::super::AuthorityAuthentication,
    pub first_event: super::super::GenerationEvent,
    pub second: super::super::ClaudeGenerationAuthority,
    pub second_authentication: super::super::AuthorityAuthentication,
    pub second_event: super::super::GenerationEvent,
}

pub(super) fn parked_pair(fixture: &Fixture) -> ParkedPair {
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &fixture.begin)
        .unwrap();
    let first_event = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
    let first_authentication = fixture.authentication();
    let first = fixture
        .store
        .apply_in_namespace(&fixture.lease, first_authentication, &first_event)
        .unwrap()
        .0;
    let second = second_begin(&fixture.begin);
    let second_authentication = fixture.authentication_for(&second);
    fixture
        .store
        .begin_in_namespace(&fixture.lease, &second)
        .unwrap();
    let second_event = fixture.start(&second.provider_session_id, SessionStartSource::Fresh);
    let second = fixture
        .store
        .apply_in_namespace(&fixture.lease, second_authentication, &second_event)
        .unwrap()
        .0;
    ParkedPair {
        first,
        first_authentication,
        first_event,
        second,
        second_authentication,
        second_event,
    }
}

fn second_begin(first: &GenerationBegin) -> GenerationBegin {
    GenerationBegin {
        invocation_id: Uuid::new_v4(),
        hook_token: Uuid::new_v4(),
        helper_capability: Uuid::new_v4(),
        process: AuthorityProcessIdentity {
            pid: SECOND_PROCESS_PID,
            start_time: SECOND_PROCESS_START,
        },
        provider_session_id: session_id(),
        ..first.clone()
    }
}
