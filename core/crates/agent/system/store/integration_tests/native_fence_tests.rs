use super::native_test_support::Fixture;
use crate::{AgentKind, AgentRecoveryAvailability};

#[path = "../test_support/native_constants.rs"]
mod constants;
use constants::*;

#[test]
fn same_content_authority_replacement_at_read_boundary_is_rejected() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        fixture.repair().unwrap();
        assert!(
            fixture
                .store
                .load_published_authoritative_with_test_hook(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime,
                    || {
                        let authority: serde_json::Value =
                            fixture.namespace.read_json(&fixture.source_path)?;
                        fixture
                            .namespace
                            .write_json(&fixture.source_path, &authority)
                    }
                )
                .is_err()
        );
    }
}

#[test]
fn source_replacement_during_projection_repair_keeps_the_binding_missing() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        let mut replaced = false;
        assert!(
            fixture
                .store
                .repair_native_binding_in_namespace_with_commit_fence(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime,
                    || {
                        if !replaced {
                            replaced = true;
                            let authority: serde_json::Value =
                                fixture.namespace.read_json(&fixture.source_path)?;
                            fixture
                                .namespace
                                .write_json(&fixture.source_path, &authority)?;
                        }
                        Ok(())
                    }
                )
                .is_err()
        );
        assert!(!fixture.store.path().exists());
    }
}

#[test]
fn previous_boot_preserves_checkpoint_but_not_live_process_authority() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::for_process(
            kind.clone(),
            crate::process::ProcessIdentity::capture(std::process::id()).unwrap(),
        );
        let mut record: serde_json::Value =
            fixture.namespace.read_json(&fixture.source_path).unwrap();
        let prior_boot = uuid::Uuid::new_v4().to_string();
        match kind {
            AgentKind::ClaudeCode => {
                for invocation in record[INVOCATIONS_FIELD].as_array_mut().unwrap() {
                    invocation[BOOT_FIELD] = serde_json::Value::String(prior_boot.clone());
                }
            }
            _ => record[CHECKPOINT_FIELD][BOOT_FIELD] = serde_json::Value::String(prior_boot),
        }
        fixture
            .namespace
            .write_json(&fixture.source_path, &record)
            .unwrap();
        let recovered = fixture.repair().unwrap().unwrap();
        assert_eq!(recovered.state, crate::AgentState::Unknown);
        assert_eq!(
            recovered.recovery_availability,
            AgentRecoveryAvailability::Available
        );
        assert!(
            fixture
                .store
                .authorize_native_resume_in_namespace(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime,
                    &recovered
                )
                .unwrap()
        );
    }
}

#[test]
fn raw_available_binding_without_its_checkpoint_cannot_authorize_resume() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        let forged = fixture.repair().unwrap().unwrap();
        std::fs::rename(
            &fixture.source_path,
            fixture.source_path.with_extension(RETAINED_EXTENSION),
        )
        .unwrap();
        assert_eq!(
            fixture.load().unwrap().unwrap().recovery_availability,
            AgentRecoveryAvailability::Unavailable
        );
        assert!(
            !fixture
                .store
                .authorize_native_resume_in_namespace(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime,
                    &forged
                )
                .unwrap()
        );
    }
}
