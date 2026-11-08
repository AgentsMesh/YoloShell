use super::native_test_support::Fixture;
use crate::{AgentBinding, AgentKind, AgentRecoveryAvailability};

const OTHER: &str = "other";
const STEP: i64 = 1;

#[test]
fn native_resume_rejects_every_foreign_subject_and_preserves_the_committed_projection() {
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        let original = fixture.repair().unwrap().unwrap();
        let before = std::fs::read(fixture.store.path()).unwrap();
        let mutations: &[(&str, fn(&mut AgentBinding))] = &[
            (stringify!(schema_version), |value| {
                value.schema_version += 1
            }),
            (stringify!(kind), |value| {
                value.kind = if value.kind == AgentKind::Codex {
                    AgentKind::ClaudeCode
                } else {
                    AgentKind::Codex
                }
            }),
            (stringify!(terminal_id), |value| {
                value.terminal_id = uuid::Uuid::new_v4()
            }),
            (stringify!(provider_session_id), |value| {
                value.provider_session_id = uuid::Uuid::new_v4().to_string()
            }),
            (stringify!(executable), |value| {
                value.executable = OTHER.into()
            }),
            (stringify!(cwd), |value| value.cwd.push_str(OTHER)),
            (stringify!(data_home), |value| {
                value.data_home.push_str(OTHER)
            }),
            (stringify!(evidence_path), |value| {
                value.evidence_path.push_str(OTHER)
            }),
            (stringify!(process_pid), |value| value.process_pid = None),
            (stringify!(process_start_time), |value| {
                value.process_start_time = None
            }),
            (stringify!(detected_at_ms), |value| {
                value.detected_at_ms += STEP
            }),
            (stringify!(recovery_availability), |value| {
                value.recovery_availability = AgentRecoveryAvailability::Unavailable
            }),
            (stringify!(state), |value| {
                value.state = crate::AgentState::Running
            }),
        ];
        assert!(
            fixture
                .store
                .authorize_native_resume_in_namespace(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime,
                    &original
                )
                .unwrap()
        );
        for (field, mutate) in mutations {
            let mut changed = original.clone();
            mutate(&mut changed);
            assert!(
                fixture
                    .store
                    .authorize_native_resume_in_namespace(
                        &fixture.namespace,
                        &fixture.context,
                        fixture.runtime,
                        &changed
                    )
                    .is_err(),
                "{field}"
            );
            assert_eq!(
                std::fs::read(fixture.store.path()).unwrap(),
                before,
                "{field}"
            );
        }
    }
}

#[test]
fn native_detection_reconciliation_checks_the_commit_fence_even_when_already_published() {
    const REJECTED: &str = "rejected detection commit";
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        let original = fixture.repair().unwrap().unwrap();
        let before = std::fs::read(fixture.store.path()).unwrap();
        let mut fence_called = false;
        let result = fixture.store.reconcile_detected_with_commit_fence(
            &fixture.namespace,
            &fixture.binding,
            fixture.runtime,
            || {
                fence_called = true;
                anyhow::bail!(REJECTED)
            },
        );
        assert!(fence_called);
        assert!(result.unwrap_err().to_string().contains(REJECTED));
        assert_eq!(std::fs::read(fixture.store.path()).unwrap(), before);
        assert_eq!(
            fixture
                .store
                .reconcile_detected_with_commit_fence(
                    &fixture.namespace,
                    &fixture.binding,
                    fixture.runtime,
                    || Ok(())
                )
                .unwrap(),
            original
        );
    }
}

#[test]
fn native_detection_repairs_missing_projection_but_never_on_a_rejected_fence() {
    const REJECTED: &str = "rejected unpublished detection";
    for kind in [AgentKind::ClaudeCode, AgentKind::Codex] {
        let fixture = Fixture::new(kind);
        assert!(!fixture.store.path().exists());
        assert!(
            fixture
                .store
                .reconcile_detected_with_commit_fence(
                    &fixture.namespace,
                    &fixture.binding,
                    fixture.runtime,
                    || anyhow::bail!(REJECTED)
                )
                .is_err()
        );
        assert!(!fixture.store.path().exists());
        let expected = fixture.load().unwrap().unwrap();
        assert_eq!(
            fixture
                .store
                .reconcile_detected_with_commit_fence(
                    &fixture.namespace,
                    &fixture.binding,
                    fixture.runtime,
                    || Ok(())
                )
                .unwrap(),
            expected
        );
        assert_eq!(
            fixture
                .store
                .load_published_authoritative_in_namespace(
                    &fixture.namespace,
                    &fixture.context,
                    fixture.runtime
                )
                .unwrap(),
            Some(expected)
        );
    }
}
