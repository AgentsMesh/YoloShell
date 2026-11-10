use super::native_test_support::Fixture;
use crate::{AgentKind, AgentRecoveryAvailability, AgentState};

#[path = "../test_support/native_constants.rs"]
mod constants;
use constants::*;

fn qwen() -> AgentKind {
    AgentKind::Other(yolo_qwen_identity::KIND.into())
}

#[test]
fn generic_hook_authority_loads_before_the_provider_creates_its_transcript() {
    let fixture = Fixture::new(qwen());
    let mut record: serde_json::Value = fixture.namespace.read_json(&fixture.source_path).unwrap();
    record[TRANSCRIPT_IDENTITY_FIELD] = serde_json::Value::Null;
    fixture
        .namespace
        .write_json(&fixture.source_path, &record)
        .unwrap();
    std::fs::remove_file(&fixture.binding.evidence_path).unwrap();

    let loaded = fixture.load().unwrap().unwrap();
    assert_eq!(loaded.state, AgentState::Unknown);
    assert_eq!(
        loaded.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
}

#[test]
fn generic_hook_authority_rejects_a_transcript_that_disappeared() {
    let fixture = Fixture::new(qwen());
    std::fs::remove_file(&fixture.binding.evidence_path).unwrap();
    assert!(fixture.load().is_err());
}

#[test]
fn generic_hook_authority_recovers_and_reauthorizes_exact_resume() {
    let fixture = Fixture::new(qwen());
    let recovered = fixture.load().unwrap().unwrap();
    assert_eq!(recovered.state, AgentState::Unknown);
    assert_eq!(
        recovered.recovery_availability,
        AgentRecoveryAvailability::Available
    );
    assert_eq!(fixture.repair().unwrap(), Some(recovered.clone()));
    assert!(
        fixture
            .store
            .authorize_native_resume_in_namespace(
                &fixture.namespace,
                &fixture.context,
                fixture.runtime,
                &recovered,
            )
            .unwrap()
    );
    let mut wrong = fixture.runtime;
    wrong.runtime_epoch = uuid::Uuid::new_v4();
    assert!(
        fixture
            .store
            .load_authoritative_in_namespace(&fixture.namespace, &fixture.context, wrong)
            .is_err()
    );
    assert!(
        !fixture
            .store
            .has_foreign_native_authority_in_namespace(&fixture.namespace, fixture.runtime)
            .unwrap()
    );
    assert!(
        fixture
            .store
            .has_foreign_native_authority_in_namespace(&fixture.namespace, wrong)
            .unwrap()
    );

    let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
        &fixture.paths,
        &fixture.namespace,
        recovered.terminal_id,
    )
    .unwrap();
    fixture
        .store
        .revoke_native_authorities_with_guard(&guard, || Ok(()))
        .unwrap();
    drop(guard);
    let revoked = fixture.load().unwrap().unwrap();
    assert_eq!(
        revoked.recovery_availability,
        AgentRecoveryAvailability::Unavailable
    );
    assert!(
        !fixture
            .store
            .authorize_native_resume_in_namespace(
                &fixture.namespace,
                &fixture.context,
                fixture.runtime,
                &recovered,
            )
            .unwrap_or(false)
    );
}

#[test]
fn generic_hook_authority_rejects_replaced_transcript_evidence() {
    let fixture = Fixture::new(qwen());
    let path = std::path::Path::new(&fixture.binding.evidence_path);
    let bytes = std::fs::read(path).unwrap();
    std::fs::rename(path, path.with_extension("retained")).unwrap();
    std::fs::write(path, bytes).unwrap();
    assert!(fixture.load().is_err());
}

#[test]
fn live_generic_hook_provider_cannot_be_resumed() {
    let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    let fixture = Fixture::for_process(qwen(), process);
    let live = fixture.repair().unwrap().unwrap();
    assert_eq!(live.state, AgentState::WaitingForInput);
    assert!(
        fixture
            .store
            .authorize_native_resume_in_namespace(
                &fixture.namespace,
                &fixture.context,
                fixture.runtime,
                &live,
            )
            .is_err()
    );
}
