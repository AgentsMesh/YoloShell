use super::{envelope, identity};
use crate::agent::hooks::EventSequence;
use crate::codex_hooks::CodexOutbox;
use tempfile::TempDir;
use uuid::Uuid;

struct Fixture {
    _directory: TempDir,
    outbox: CodexOutbox,
    occurrence: crate::agent::hooks::HookEnvelope,
    assigned: crate::agent::hooks::HookEnvelope,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let identity = identity();
        let outbox =
            CodexOutbox::for_identity(directory.path().join("outbox.json"), &identity.hook)
                .unwrap();
        let occurrence = envelope(&identity, Uuid::new_v4());
        let assigned = outbox.enqueue(occurrence.clone()).unwrap();
        Self {
            _directory: directory,
            outbox,
            occurrence,
            assigned,
        }
    }

    fn assert_immutable(
        &self,
        query: impl FnOnce(&CodexOutbox) -> anyhow::Result<Option<crate::agent::hooks::HookEnvelope>>,
    ) -> anyhow::Result<Option<crate::agent::hooks::HookEnvelope>> {
        let before = snapshot(&self.outbox);
        let result = query(&self.outbox);
        assert_eq!(snapshot(&self.outbox), before);
        result
    }
}

#[test]
fn pending_and_acknowledged_exact_occurrences_are_authoritative() {
    let fixture = Fixture::new();
    let pending = fixture
        .assert_immutable(|outbox| outbox.authoritative_occurrence(&fixture.occurrence))
        .unwrap();
    assert_eq!(pending, Some(fixture.assigned.clone()));

    fixture
        .outbox
        .acknowledge(fixture.assigned.sequence.0, fixture.assigned.event_id)
        .unwrap();
    let acknowledged = fixture
        .assert_immutable(|outbox| outbox.authoritative_occurrence(&fixture.occurrence))
        .unwrap();
    assert_eq!(acknowledged, Some(fixture.assigned.clone()));
}

#[test]
fn unknown_occurrences_have_no_authority() {
    let fixture = Fixture::new();
    let mut unknown = fixture.occurrence.clone();
    unknown.event_id = Uuid::new_v4();
    assert_eq!(
        fixture
            .assert_immutable(|outbox| outbox.authoritative_occurrence(&unknown))
            .unwrap(),
        None
    );
}

#[test]
fn changed_occurrences_fail_closed() {
    let fixture = Fixture::new();
    let mut changed = fixture.occurrence.clone();
    changed.provider_session_id.push_str("-changed");
    assert!(
        fixture
            .assert_immutable(|outbox| outbox.authoritative_occurrence(&changed))
            .is_err()
    );
}

#[test]
fn malformed_occurrence_queries_fail_without_mutating_the_outbox() {
    let fixture = Fixture::new();
    let mut nil = fixture.occurrence.clone();
    nil.event_id = Uuid::nil();
    assert!(
        fixture
            .assert_immutable(|outbox| outbox.authoritative_occurrence(&nil))
            .is_err()
    );

    let mut assigned = fixture.occurrence.clone();
    assigned.sequence = EventSequence::FIRST;
    assert!(
        fixture
            .assert_immutable(|outbox| outbox.authoritative_occurrence(&assigned))
            .is_err()
    );
}

fn snapshot(outbox: &CodexOutbox) -> (Vec<u8>, crate::paths::identity::FileIdentity) {
    let path = outbox.path();
    let bytes = std::fs::read(path).unwrap();
    let identity = crate::paths::identity::FileIdentity::inspect_bounded(
        &std::fs::metadata(path).unwrap(),
        path,
        crate::codex_hooks::MAX_OUTBOX_BYTES as u64,
    )
    .unwrap();
    (bytes, identity)
}
