use super::native_test_support::Fixture;
use crate::{AgentKind, AgentRecoveryAvailability, AgentState};

const REPLACED_DIRECTORY: &str = "replaced-project";

#[test]
fn missing_projection_is_readable_and_repaired_without_guessing_completion() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        assert!(!fixture.store.path().exists());
        let recovered = fixture.load().unwrap().unwrap();
        assert_eq!(recovered.state, AgentState::Unknown);
        assert_eq!(
            recovered.recovery_availability,
            AgentRecoveryAvailability::Available
        );
        assert_eq!(fixture.repair().unwrap(), Some(recovered.clone()));
        assert_eq!(
            fixture
                .store
                .read_raw_in_namespace(&fixture.namespace)
                .unwrap(),
            Some(recovered.clone())
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
fn wrong_runtime_and_superseded_process_do_not_inherit_checkpoint_authority() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        fixture.repair().unwrap();
        let mut wrong = fixture.runtime;
        wrong.runtime_epoch = uuid::Uuid::new_v4();
        assert!(
            fixture
                .store
                .load_authoritative_in_namespace(&fixture.namespace, &fixture.context, wrong)
                .is_err()
        );
        let mut superseded = fixture
            .store
            .read_raw_in_namespace(&fixture.namespace)
            .unwrap()
            .unwrap();
        superseded.process_start_time = Some(u64::MAX);
        fixture
            .namespace
            .write_json(fixture.store.path(), &superseded)
            .unwrap();
        let loaded = fixture.load().unwrap().unwrap();
        assert_eq!(
            loaded.recovery_availability,
            AgentRecoveryAvailability::Unavailable
        );
        assert!(fixture.repair().unwrap().is_none());
    }
}

#[test]
fn transcript_replacement_and_same_path_directory_replacement_revoke_resume_evidence() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind.clone());
        fixture.repair().unwrap();
        let path = std::path::Path::new(&fixture.binding.evidence_path);
        let bytes = std::fs::read(path).unwrap();
        std::fs::rename(path, path.with_extension("retained")).unwrap();
        std::fs::write(path, bytes).unwrap();
        assert!(fixture.load().is_err());

        let fixture = Fixture::new(kind);
        let cwd = std::path::Path::new(&fixture.binding.cwd);
        std::fs::rename(cwd, cwd.with_file_name(REPLACED_DIRECTORY)).unwrap();
        std::fs::create_dir(cwd).unwrap();
        assert!(fixture.load().is_err());
    }
}

#[test]
fn live_provider_cannot_be_resumed_even_with_verified_checkpoint() {
    let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::for_process(kind, process);
        let live = fixture.repair().unwrap().unwrap();
        assert_eq!(live.state, AgentState::Running);
        assert!(
            fixture
                .store
                .authorize_native_resume_in_namespace(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime,
                    &live
                )
                .is_err()
        );
    }
}

#[test]
fn explicit_recreation_revokes_durable_checkpoint_before_binding_clear() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        let available = fixture.repair().unwrap().unwrap();
        let guard = crate::paths::durable_mutation::SessionDurableMutationGuard::acquire(
            &fixture.paths,
            &fixture.namespace,
            available.terminal_id,
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
            fixture
                .store
                .authorize_native_resume_in_namespace(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime,
                    &available
                )
                .is_err()
        );
    }
}
