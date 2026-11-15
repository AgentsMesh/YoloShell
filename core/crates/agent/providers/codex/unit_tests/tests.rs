use super::*;
use crate::codex::test_support::{self, Fixture};
use std::path::PathBuf;

const NODE_ENTRYPOINT: &str = "/opt/node_modules/@openai/codex/bin/codex.js";
const IMPOSITOR_EXECUTABLE: &str = "codex-helper";
const INVALID_SESSION_ID: &str = "session";

#[test]
fn adapter_matches_only_codex_process_shapes() {
    let fixture = Fixture::new();
    let adapter = CodexAdapter;
    let direct = fixture.process(
        test_support::PROCESS_PID,
        test_support::PROCESS_START_TIME,
        &[],
        None,
    );
    assert!(adapter.matches_process(&direct));

    let mut node = direct.clone();
    node.executable = constants::NODE_EXECUTABLE.into();
    node.argv = vec![constants::NODE_EXECUTABLE.into(), NODE_ENTRYPOINT.into()];
    assert!(adapter.matches_process(&node));

    let mut impostor = direct;
    impostor.executable = IMPOSITOR_EXECUTABLE.into();
    impostor.argv = vec![IMPOSITOR_EXECUTABLE.into()];
    assert!(!adapter.matches_process(&impostor));
}

#[test]
fn context_inspection_and_resume_plan_preserve_provider_identity() {
    let fixture = Fixture::new();
    let evidence_path = fixture.write_main(test_support::SESSION_ID);
    let adapter = CodexAdapter;
    assert!(
        adapter
            .inspect_evidence(&evidence_path, &fixture.cwd)
            .unwrap()
            .is_none()
    );
    let evidence = fixture.inspect(&evidence_path).unwrap();
    assert_eq!(evidence.provider_session_id, test_support::SESSION_ID);
    assert_eq!(evidence.cwd, fixture.cwd.canonicalize().unwrap());
    assert_eq!(evidence.data_home, fixture.data_home);
    assert_eq!(evidence.state, AgentState::Unknown);

    let binding = binding(&fixture, &evidence);
    assert_eq!(
        adapter.resume_plan(&binding),
        ResumePlan {
            executable: constants::EXECUTABLE.into(),
            args: vec![
                constants::RESUME_ARGUMENT.into(),
                constants::CWD_ARGUMENT.into(),
                binding.cwd.clone(),
                test_support::SESSION_ID.into(),
            ],
            environment: [(
                constants::DATA_HOME_ENV.to_owned(),
                fixture.data_home.display().to_string(),
            )]
            .into(),
        }
    );
    adapter
        .validate_binding(&binding, &fixture.context())
        .unwrap();
}

#[test]
fn binding_validation_rejects_noncanonical_id_and_other_data_home() {
    let fixture = Fixture::new();
    let evidence = fixture
        .inspect(&fixture.write_main(test_support::SESSION_ID))
        .unwrap();
    let mut binding = binding(&fixture, &evidence);
    binding.provider_session_id = INVALID_SESSION_ID.into();
    assert!(
        CodexAdapter
            .validate_binding(&binding, &fixture.context())
            .is_err()
    );

    binding.provider_session_id = test_support::SESSION_ID.into();
    binding.data_home = fixture.cwd.display().to_string();
    assert!(
        CodexAdapter
            .validate_binding(&binding, &fixture.context())
            .is_err()
    );
}

fn binding(fixture: &Fixture, evidence: &AgentEvidence) -> AgentBinding {
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: fixture.terminal_id,
        kind: AgentKind::Codex,
        provider_session_id: evidence.provider_session_id.clone(),
        executable: constants::EXECUTABLE.into(),
        cwd: evidence.cwd.display().to_string(),
        evidence_path: evidence.path.display().to_string(),
        data_home: evidence.data_home.display().to_string(),
        process_pid: Some(test_support::PROCESS_PID),
        process_start_time: Some(test_support::PROCESS_START_TIME),
        state: evidence.state,
        recovery_availability: crate::agent::AgentRecoveryAvailability::UnmanagedLaunch,
        detected_at_ms: 1,
        updated_at_ms: 1,
    }
}

#[test]
fn invalid_rollout_layout_cannot_borrow_a_context_data_home() {
    let fixture = Fixture::new();
    let outside = fixture.cwd.join("rollout.jsonl");
    std::fs::write(&outside, "{}\n").unwrap();
    assert!(fixture.inspect(&outside).is_none());
    let wrong_context = DetectionContext::new(
        fixture.terminal_id,
        fixture.cwd.clone(),
        [(AgentKind::Codex, PathBuf::from("/unrelated-codex-home"))],
    );
    let evidence = fixture.write_main(test_support::SESSION_ID);
    assert!(
        CodexAdapter
            .inspect_evidence_in_context(&evidence, &fixture.cwd, &wrong_context)
            .unwrap()
            .is_none()
    );
}
