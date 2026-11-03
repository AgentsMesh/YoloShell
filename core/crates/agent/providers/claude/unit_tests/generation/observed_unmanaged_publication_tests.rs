use super::super::{ClaudeGenerationAuthority, SessionStartSource};
use super::observed_unmanaged_publication_test_support::*;
use super::support::Fixture;
use crate::agent::{AgentBinding, AgentBindingStore, AgentRecoveryAvailability, ObservedProcess};
use anyhow::Result;

#[test]
fn scanner_wins_before_session_start_is_promoted() {
    let case = Case::new(ABSOLUTE_CLAUDE, vec![ABSOLUTE_CLAUDE.to_owned()]);

    let (committed, promoted) = case.publish().unwrap();

    assert!(committed.matches_binding(&promoted));
    assert_eq!(case.binding_at(), promoted);
    assert_eq!(
        promoted.recovery_availability,
        AgentRecoveryAvailability::Available
    );
}

#[test]
fn managed_node_entrypoint_is_promoted() {
    let case = Case::new(
        ABSOLUTE_NODE,
        vec![ABSOLUTE_NODE.to_owned(), CLAUDE_ENTRYPOINT.to_owned()],
    );

    let (committed, promoted) = case.publish().unwrap();

    assert!(committed.matches_binding(&promoted));
    assert_eq!(case.binding_at(), promoted);
}

#[test]
fn initial_scan_without_transcript_hint_is_promoted() {
    let mut case = Case::native();
    case.observed.evidence_paths = None;

    let (committed, promoted) = case.publish().unwrap();

    assert!(committed.matches_binding(&promoted));
    assert_eq!(case.binding_at(), promoted);
}

#[test]
fn ordinary_publication_cannot_promote_an_unmanaged_scan() {
    let case = Case::native();
    let authority_before = case
        .fixture
        .store
        .load_in_namespace(&case.fixture.lease)
        .unwrap();

    assert!(
        case.fixture
            .store
            .publish_binding_state_in_namespace(
                &case.fixture.lease,
                &case.fixture.binding_path(),
                case.fixture.authentication(),
                &case.authority,
                false,
                |_| Ok(()),
            )
            .is_err()
    );
    assert_eq!(case.binding_at(), case.scanned);
    assert_eq!(
        case.fixture
            .store
            .load_in_namespace(&case.fixture.lease)
            .unwrap(),
        authority_before
    );
}

#[test]
fn foreign_binding_fields_fail_closed() {
    for mutate in binding_mutations() {
        let mut case = Case::native();
        mutate(&mut case.observed, &mut case.scanned);
        case.assert_rejected();
    }
}

#[test]
fn foreign_process_proofs_fail_closed() {
    for mutate in process_mutations() {
        let mut case = Case::native();
        mutate(&case.authority, &mut case.observed);
        case.assert_rejected();
    }
}

struct Case {
    fixture: Fixture,
    authority: ClaudeGenerationAuthority,
    observed: ObservedProcess,
    scanned: AgentBinding,
}

impl Case {
    fn native() -> Self {
        Self::new(ABSOLUTE_CLAUDE, vec![ABSOLUTE_CLAUDE.to_owned()])
    }

    fn new(executable: &str, argv: Vec<String>) -> Self {
        let fixture = Fixture::new();
        let initial = fixture
            .store
            .begin_in_namespace(&fixture.lease, &fixture.begin)
            .unwrap();
        let event = fixture.start(&fixture.begin.provider_session_id, SessionStartSource::Fresh);
        let identity = event_identity(&fixture, &event);
        let (authority, _, _) = fixture
            .store
            .transition_foreground_durable_in_namespace(
                &fixture.lease,
                fixture.authentication(),
                &initial,
                initial
                    .last_event_sequence
                    .checked_add(super::super::constants::REVISION_INCREMENT)
                    .unwrap(),
                &event,
                Some(identity),
            )
            .unwrap();
        let transcript = authority.transcript_path.clone().unwrap();
        let observed = observed(&authority, executable, argv, transcript);
        let scanned = scanned(&authority, &observed);
        fixture
            .lease
            .write_json(&fixture.binding_path(), &scanned)
            .unwrap();
        Self {
            fixture,
            authority,
            observed,
            scanned,
        }
    }

    fn publish(&self) -> anyhow::Result<(ClaudeGenerationAuthority, AgentBinding)> {
        let admission =
            super::super::observed_unmanaged_publication::admit_observed_publication_for_test(
                &self.authority,
                self.observed.clone(),
                AcceptingFence,
            )?;
        self.fixture.store.publish_hook_binding_state_in_namespace(
            &self.fixture.lease,
            &self.fixture.binding_path(),
            self.fixture.authentication(),
            &self.authority,
            false,
            &admission,
            |_| Ok(()),
        )
    }

    fn binding_at(&self) -> AgentBinding {
        AgentBindingStore::new(self.fixture.binding_path())
            .read_raw_in_namespace(&self.fixture.lease)
            .unwrap()
            .unwrap()
    }

    fn assert_rejected(&self) {
        self.fixture
            .lease
            .write_json(&self.fixture.binding_path(), &self.scanned)
            .unwrap();
        let authority_before = self
            .fixture
            .store
            .load_in_namespace(&self.fixture.lease)
            .unwrap();
        assert!(self.publish().is_err());
        assert_eq!(self.binding_at(), self.scanned);
        assert_eq!(
            self.fixture
                .store
                .load_in_namespace(&self.fixture.lease)
                .unwrap(),
            authority_before
        );
    }
}

struct AcceptingFence;

impl super::super::HookPublicationFence for AcceptingFence {
    fn verify(&self, _: &ClaudeGenerationAuthority) -> Result<()> {
        Ok(())
    }
}
