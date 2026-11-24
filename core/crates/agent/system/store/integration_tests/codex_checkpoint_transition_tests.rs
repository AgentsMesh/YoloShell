use super::native_test_support::Fixture;
use crate::codex::authority::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::{AgentKind, AgentRecoveryAvailability, AgentState};

const VERSION_ADVANCE: u64 = 1;

#[test]
fn unavailable_projection_must_explicitly_carry_the_verified_checkpoint() {
    let fixture = Fixture::new(AgentKind::Codex);
    let store = fixture.store.codex_authority_store().unwrap();
    let original = store
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    let mut binding = original.binding.clone();
    binding.state = AgentState::Unknown;
    binding.recovery_availability = AgentRecoveryAvailability::Unavailable;
    let missing = CodexBindingAuthority::new(
        fixture.runtime,
        original.provider_process(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        original.version,
        CodexAuthorityStatus::Unavailable,
        binding,
    )
    .unwrap();
    assert!(
        store
            .publish_in_namespace_with_commit_fence(
                &fixture.namespace,
                fixture.store.path(),
                &missing,
                || Ok(())
            )
            .is_err()
    );
    let retained = missing.inherit_verified_resume_identity(&original).unwrap();
    store
        .publish_in_namespace_with_commit_fence(
            &fixture.namespace,
            fixture.store.path(),
            &retained,
            || Ok(()),
        )
        .unwrap();
    assert!(
        retained
            .recoverable_binding(fixture.runtime)
            .unwrap()
            .is_some()
    );
}

#[test]
fn advancing_an_authority_revision_cannot_silently_drop_its_checkpoint() {
    let fixture = Fixture::new(AgentKind::Codex);
    let store = fixture.store.codex_authority_store().unwrap();
    let original = store
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    let mut version = original.version;
    version.sequence += VERSION_ADVANCE;
    version.authority_revision += VERSION_ADVANCE;
    let missing = CodexBindingAuthority::new(
        fixture.runtime,
        original.provider_process(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        version,
        original.status,
        original.binding.clone(),
    )
    .unwrap();
    assert!(
        store
            .publish_in_namespace_with_commit_fence(
                &fixture.namespace,
                fixture.store.path(),
                &missing,
                || Ok(())
            )
            .is_err()
    );
    let retained = missing.inherit_verified_resume_identity(&original).unwrap();
    store
        .publish_in_namespace_with_commit_fence(
            &fixture.namespace,
            fixture.store.path(),
            &retained,
            || Ok(()),
        )
        .unwrap();
    assert_eq!(
        store.load_in_namespace(&fixture.namespace).unwrap(),
        Some(retained)
    );
}
