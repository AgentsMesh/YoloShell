use super::super::{AuthorityProcessIdentity, GenerationBegin};
use super::recovery_orphan_test_support::ParkedRecoveryFixture;
use super::support::session_id;

const EXISTING_AUTHORITIES: usize = 2;
const IDENTITY_INCREMENT: u32 = 1;

#[test]
fn recoverable_pending_queue_is_reconciled_before_capacity_admission() {
    let fixture = ParkedRecoveryFixture::new();
    let _guards = fixture.gone_guards();
    let (candidate, _) = fill(&fixture);
    let admitted = fixture.begin(&candidate).unwrap();
    assert_eq!(admitted.invocation_id, candidate.invocation_id);
    assert_recovery_retired(&fixture);
}

#[test]
fn empty_dead_parked_authority_is_retired_before_capacity_admission() {
    let fixture = ParkedRecoveryFixture::new();
    let _guards = fixture.gone_guards();
    fixture.acknowledge_without_retire();
    let (candidate, _) = fill(&fixture);
    let admitted = fixture.begin(&candidate).unwrap();
    assert_eq!(admitted.invocation_id, candidate.invocation_id);
    assert_recovery_retired(&fixture);
}

#[test]
fn unknown_parked_process_blocks_reconciliation_and_capacity_admission() {
    let fixture = ParkedRecoveryFixture::new();
    let (candidate, mut retained) = fill(&fixture);
    retained.push(fixture.pair.first.process);
    let _guards = retained
        .into_iter()
        .map(|identity| {
            crate::process::install_test_process_identity_relation(
                identity.pid,
                identity.start_time,
                crate::process::ProcessIdentityRelation::Unknown,
            )
        })
        .collect::<Vec<_>>();
    let error = fixture.begin(&candidate).unwrap_err();
    assert!(
        error
            .to_string()
            .contains(super::super::constants::AUTHORITY_SET_CAPACITY)
    );
    assert!(
        fixture
            .base
            .store
            .load_invocation_in_namespace(&fixture.base.lease, fixture.pair.first.invocation_id,)
            .unwrap()
            .is_some()
    );
    assert!(matches!(
        crate::claude_hooks::outbox::probe_recovery(&fixture.outbox, fixture.credentials).unwrap(),
        crate::claude_hooks::outbox::RecoveryProbe::Blocked
    ));
}

#[test]
fn stale_active_transcript_does_not_block_production_begin_admission() {
    for remove in [false, true] {
        let fixture = ParkedRecoveryFixture::new();
        let transcript = fixture.pair.second.transcript_path.as_ref().unwrap();
        if remove {
            std::fs::remove_file(transcript).unwrap();
        } else {
            std::fs::write(transcript, []).unwrap();
        }
        assert!(
            fixture
                .binding_store
                .load_authoritative_in_namespace(
                    &fixture.base.lease,
                    &fixture.context,
                    fixture.base.runtime(),
                )
                .is_err()
        );
        let candidate = begin(&fixture, EXISTING_AUTHORITIES);
        let admitted = fixture.begin(&candidate).unwrap();
        assert_eq!(admitted.invocation_id, candidate.invocation_id);
    }
}

fn fill(fixture: &ParkedRecoveryFixture) -> (GenerationBegin, Vec<AuthorityProcessIdentity>) {
    let mut retained = vec![fixture.pair.second.process];
    let mut candidate = None;
    for ordinal in EXISTING_AUTHORITIES..=super::super::constants::MAX_INVOCATION_AUTHORITIES {
        let begin = begin(fixture, ordinal);
        if ordinal == super::super::constants::MAX_INVOCATION_AUTHORITIES {
            candidate = Some(begin);
        } else {
            retained.push(begin.process);
            fixture
                .base
                .store
                .begin_in_namespace(&fixture.base.lease, &begin)
                .unwrap();
        }
    }
    (candidate.unwrap(), retained)
}

fn assert_recovery_retired(fixture: &ParkedRecoveryFixture) {
    assert!(
        fixture
            .base
            .store
            .load_invocation_in_namespace(&fixture.base.lease, fixture.pair.first.invocation_id,)
            .unwrap()
            .is_none()
    );
    assert!(
        crate::claude_hooks::outbox::pending_recovery(&fixture.outbox, fixture.credentials,)
            .unwrap()
            .is_none()
    );
}

fn begin(fixture: &ParkedRecoveryFixture, ordinal: usize) -> GenerationBegin {
    let offset = u32::try_from(ordinal)
        .unwrap()
        .checked_add(IDENTITY_INCREMENT)
        .unwrap();
    GenerationBegin {
        invocation_id: uuid::Uuid::new_v4(),
        hook_token: uuid::Uuid::new_v4(),
        helper_capability: uuid::Uuid::new_v4(),
        process: AuthorityProcessIdentity {
            pid: fixture.pair.second.process.pid.checked_add(offset).unwrap(),
            start_time: fixture
                .pair
                .second
                .process
                .start_time
                .checked_add(u64::from(offset))
                .unwrap(),
        },
        provider_session_id: session_id(),
        ..fixture.base.begin.clone()
    }
}
