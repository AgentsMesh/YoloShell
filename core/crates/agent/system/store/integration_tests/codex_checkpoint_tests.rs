use super::native_test_support::Fixture;
use crate::codex::authority::{CodexAuthorityStatus, CodexBindingAuthority};
use crate::{AgentKind, AgentRecoveryAvailability, AgentState};

#[path = "../test_support/native_constants.rs"]
mod constants;
use constants::*;

#[path = "../test_support/codex_resume_consistency.rs"]
mod resume_consistency_support;
#[path = "codex_resume_consistency_tests.rs"]
mod resume_consistency_tests;
#[path = "codex_resume_source_transition_tests.rs"]
mod resume_source_transition_tests;

#[test]
fn same_version_cannot_remove_or_substitute_checkpoint_metadata() {
    let fixture = Fixture::new(AgentKind::Codex);
    let store = fixture.store.codex_authority_store().unwrap();
    let original = store
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    let cases = [
        {
            let mut value = serde_json::to_value(&original).unwrap();
            value.as_object_mut().unwrap().remove(CHECKPOINT_FIELD);
            value
        },
        {
            let mut value = serde_json::to_value(&original).unwrap();
            value[CHECKPOINT_FIELD][BOOT_FIELD] = uuid::Uuid::new_v4().to_string().into();
            value
        },
        {
            let mut value = serde_json::to_value(&original).unwrap();
            value[CHECKPOINT_FIELD][BINDING_FIELD][STATE_FIELD] = UNKNOWN_STATE.into();
            value
        },
    ];
    for value in cases {
        let missing = value.get(CHECKPOINT_FIELD).is_none();
        let decoded = serde_json::from_value::<CodexBindingAuthority>(value);
        if missing {
            assert!(decoded.is_err());
        } else {
            assert!(
                store
                    .publish_in_namespace_with_commit_fence(
                        &fixture.namespace,
                        fixture.store.path(),
                        &decoded.unwrap(),
                        || Ok(())
                    )
                    .is_err()
            );
        }
        assert_eq!(
            store.load_in_namespace(&fixture.namespace).unwrap(),
            Some(original.clone())
        );
    }
}

#[test]
fn unavailable_status_keeps_unknown_activity_and_verified_resume() {
    let fixture = Fixture::new(AgentKind::Codex);
    fixture.repair().unwrap();
    let store = fixture.store.codex_authority_store().unwrap();
    let unavailable = store
        .invalidate_in_namespace_with_commit_fence(
            &fixture.namespace,
            fixture.store.path(),
            fixture.runtime,
            || Ok(()),
        )
        .unwrap()
        .unwrap();
    assert_eq!(unavailable.status, CodexAuthorityStatus::Unavailable);
    let recoverable = unavailable
        .recoverable_binding(fixture.runtime)
        .unwrap()
        .unwrap();
    assert_eq!(recoverable.state, AgentState::Unknown);
    assert_eq!(
        recoverable.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert_eq!(fixture.load().unwrap(), Some(recoverable));
}

#[test]
fn same_version_cannot_replace_runtime_or_exact_provider_process() {
    let fixture = Fixture::new(AgentKind::Codex);
    let store = fixture.store.codex_authority_store().unwrap();
    let original = store
        .load_in_namespace(&fixture.namespace)
        .unwrap()
        .unwrap();
    let mut wrong = fixture.runtime;
    wrong.runtime_epoch = uuid::Uuid::new_v4();
    let replacement = CodexBindingAuthority::new(
        wrong,
        original.provider_process(),
        crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
        original.version,
        original.status,
        original.binding.clone(),
    )
    .unwrap()
    .with_verified_resume_identity(&crate::process::boot_id())
    .unwrap();
    assert!(
        store
            .publish_in_namespace_with_commit_fence(
                &fixture.namespace,
                fixture.store.path(),
                &replacement,
                || Ok(())
            )
            .is_err()
    );
}

#[test]
fn revoked_checkpoint_cannot_republish_a_cleared_binding() {
    let fixture = Fixture::new(AgentKind::Codex);
    let store = fixture.store.codex_authority_store().unwrap();
    let (authority, identity): (CodexBindingAuthority, _) = fixture
        .namespace
        .read_json_with_identity(store.path())
        .unwrap();
    store
        .revoke_exact_in_namespace_with_commit_fence(
            &fixture.namespace,
            &authority,
            identity,
            || Ok(()),
        )
        .unwrap();
    let revoked: (CodexBindingAuthority, _) = fixture
        .namespace
        .read_json_with_identity(store.path())
        .unwrap();
    assert_eq!(revoked.0.status, CodexAuthorityStatus::Revoked);
    assert!(revoked.0.resume_boot_id().is_some());
    assert!(
        fixture
            .namespace
            .file_is_missing(fixture.store.path())
            .unwrap()
    );
    assert!(fixture.repair().unwrap().is_none());
    assert!(fixture.load().unwrap().is_none());
    assert!(
        fixture
            .namespace
            .file_is_missing(fixture.store.path())
            .unwrap()
    );
    assert_eq!(
        fixture
            .namespace
            .read_json_with_identity::<CodexBindingAuthority>(store.path())
            .unwrap(),
        revoked
    );
}
