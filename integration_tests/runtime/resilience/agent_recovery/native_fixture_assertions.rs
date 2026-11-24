use super::*;
use yolo_runtime::agent::claude::generation::{AuthorityProcessIdentity, AuthorityRuntimeIdentity};
use yolo_runtime::agent::{AgentKind, AgentRecoveryAvailability};

const NAMESPACE_LABEL: &str = "native recovery matrix prepared fixture";

pub(super) fn assert_prepared(
    paths: &Paths,
    runtime: &RuntimeRecord,
    fixture: &matrix::RecoveryFixture,
) {
    let path = paths.agent_binding(runtime.terminal_id);
    let namespace = yolo_runtime::namespace_lease::NamespaceLease::capture(&[(
        NAMESPACE_LABEL,
        &paths.session_dir(runtime.terminal_id),
    )])
    .unwrap();
    let original = fixture.previous.agent.as_ref().unwrap();
    let projected = match original.kind {
        AgentKind::ClaudeCode => {
            let store = yolo_runtime::agent::claude::generation::native::ClaudeNativeGenerationStore::from_binding_path(&path).unwrap();
            let record = store.load_in_namespace(&namespace).unwrap().unwrap();
            if fixture.revoked() {
                use yolo_runtime::agent::claude::generation_test_hook;
                generation_test_hook::persist_native_revoked(&path).unwrap();
                assert!(
                    generation_test_hook::persist_native_durable_state(&path, AgentState::Running)
                        .is_err()
                );
            }
            assert!(
                yolo_runtime::agent::claude::generation_test_hook::persist_native_previous_boot(
                    &path,
                    Uuid::new_v4(),
                    PREVIOUS_BOOT_ID,
                    &runtime.boot_id,
                )
                .is_err()
            );
            assert_eq!(
                store.load_in_namespace(&namespace).unwrap().as_ref(),
                Some(&record)
            );
            assert_eq!(record.boot_id, PREVIOUS_BOOT_ID);
            assert_eq!(record.scope.runtime_epoch, runtime.runtime_epoch);
            assert_eq!(record.scope.agent.pid, original.process_pid.unwrap());
            assert_eq!(
                record.scope.agent.start_time,
                original.process_start_time.unwrap()
            );
            assert_eq!(record.revoked, fixture.revoked());
            assert_eq!(
                record.final_session_end,
                fixture.state == AgentState::Exited
            );
            assert_eq!(record.verify_resume_evidence().unwrap(), !fixture.revoked());
            let projection = record.recovery_binding(false);
            assert_eq!(projection.state, AgentState::Unknown);
            projection
        }
        AgentKind::Codex => {
            let store = yolo_runtime::agent::codex::authority::CodexBindingAuthorityStore::from_binding_path(&path).unwrap();
            let record = store.load_in_namespace(&namespace).unwrap().unwrap();
            if fixture.revoked() {
                use yolo_runtime::agent::codex::authority_test_hook;
                authority_test_hook::persist_native_revoked(&path).unwrap();
                assert!(
                    authority_test_hook::persist_native_durable_state(&path, AgentState::Running)
                        .is_err()
                );
            }
            assert!(
                yolo_runtime::agent::codex::authority_test_hook::persist_native_previous_boot(
                    &path,
                    Uuid::new_v4(),
                    PREVIOUS_BOOT_ID,
                    &runtime.boot_id,
                )
                .is_err()
            );
            assert_eq!(
                store.load_in_namespace(&namespace).unwrap().as_ref(),
                Some(&record)
            );
            assert_eq!(record.resume_boot_id(), Some(PREVIOUS_BOOT_ID));
            assert_eq!(record.provider_process().pid, original.process_pid.unwrap());
            assert_eq!(
                record.provider_process().start_time,
                original.process_start_time.unwrap()
            );
            record.verify_resume_evidence().unwrap();
            let identity = AuthorityRuntimeIdentity {
                runtime_epoch: runtime.runtime_epoch,
                worker: AuthorityProcessIdentity {
                    pid: runtime.worker_pid,
                    start_time: runtime.worker_start_time,
                },
            };
            let recovered = record.recoverable_binding(identity).unwrap();
            assert_eq!(recovered.is_none(), fixture.revoked());
            let projection = recovered.unwrap_or(record.binding);
            assert_eq!(projection.state, fixture.state);
            if fixture.state == AgentState::Exited {
                assert!(projection.process_pid.is_none());
                assert!(projection.process_start_time.is_none());
            }
            projection
        }
        AgentKind::Other(_) => unreachable!(),
    };
    assert_eq!(
        projected.recovery_availability,
        if fixture.revoked() {
            AgentRecoveryAvailability::Unavailable
        } else {
            AgentRecoveryAvailability::Available
        }
    );
    assert_eq!(projected.provider_session_id, original.provider_session_id);
    assert_eq!(projected.evidence_path, original.evidence_path);
    assert_eq!(projected.cwd, original.cwd);
    assert_eq!(projected.data_home, original.data_home);
    assert_eq!(projected.detected_at_ms, original.detected_at_ms);
}
