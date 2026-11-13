use super::*;
use std::cell::Cell;

const AUTHORITY_COMMIT: usize = 1;
const BINDING_COMMIT: usize = 2;
const COMMIT_INCREMENT: usize = 1;
const INJECTED_COMMIT_FAILURE: &str = "injected Codex authority commit failure";
const ACTIVE_AUTHORITY_EXPECTED: &str = "active authority must invalidate";

#[test]
fn authority_commit_fence_failure_preserves_the_active_pair() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();

    assert!(invalidate_with_fault(&fixture, AUTHORITY_COMMIT).is_err());
    assert_pair(&fixture, &active);
}

#[test]
fn binding_commit_fence_failure_is_repaired_by_idempotent_invalidation() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    fixture.publish(&active).unwrap();

    assert!(invalidate_with_fault(&fixture, BINDING_COMMIT).is_err());
    let unavailable = fixture
        .store()
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    assert_eq!(unavailable.status, CodexAuthorityStatus::Unavailable);
    assert_eq!(read_binding(&fixture), active.binding);

    let repaired = fixture.invalidate(fixture.runtime).unwrap().unwrap();
    assert_eq!(repaired, unavailable);
    assert_pair(&fixture, &repaired);
}

#[test]
fn exhausted_authority_repairs_a_binding_left_at_the_prior_commit() {
    let fixture = Fixture::new();
    let active = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Active);
    let exhausted = fixture.authority(Generation::INITIAL.0, CodexAuthorityStatus::Exhausted);
    fixture.publish(&active).unwrap();
    crate::paths::atomic_write_json(&authority_path(&fixture), &exhausted).unwrap();

    let repaired = fixture.invalidate(fixture.runtime).unwrap().unwrap();
    assert_eq!(repaired, exhausted);
    assert_pair(&fixture, &repaired);
}

fn invalidate_with_fault(fixture: &Fixture, fail_at: usize) -> anyhow::Result<()> {
    let calls = Cell::new(usize::default());
    fixture
        .store()
        .invalidate_in_namespace_with_commit_fence(
            &fixture.namespace,
            &fixture.binding_path,
            fixture.runtime,
            || {
                let call = calls.get() + COMMIT_INCREMENT;
                calls.set(call);
                anyhow::ensure!(call != fail_at, INJECTED_COMMIT_FAILURE);
                Ok(())
            },
        )?
        .expect(ACTIVE_AUTHORITY_EXPECTED);
    Ok(())
}

fn assert_pair(fixture: &Fixture, expected: &CodexBindingAuthority) {
    assert_eq!(
        fixture
            .store()
            .load_in_namespace(&fixture.namespace)
            .unwrap()
            .as_ref(),
        Some(expected)
    );
    assert_eq!(&read_binding(fixture), &expected.binding);
}

fn read_binding(fixture: &Fixture) -> AgentBinding {
    fixture.namespace.read_json(&fixture.binding_path).unwrap()
}

fn authority_path(fixture: &Fixture) -> std::path::PathBuf {
    fixture
        .binding_path
        .parent()
        .unwrap()
        .join(AUTHORITY_FILE_NAME)
}
