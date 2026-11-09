use super::*;
use anyhow::{Result, anyhow};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;
use uuid::Uuid;

fn context(cwd: PathBuf) -> DetectionContext {
    DetectionContext::new(Uuid::new_v4(), cwd.clone(), [(AgentKind::Codex, cwd)])
}
fn binding(kind: AgentKind, path: &Path, state: AgentState) -> AgentBinding {
    AgentBinding {
        schema_version: BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind,
        provider_session_id: "session".into(),
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        cwd: "/workspace".into(),
        evidence_path: path.display().to_string(),
        data_home: "/home".into(),
        process_pid: (state != AgentState::Exited).then_some(21),
        process_start_time: (state != AgentState::Exited).then_some(21),
        state,
        recovery_availability: crate::agent::AgentRecoveryAvailability::Available,
        detected_at_ms: 7,
        updated_at_ms: 8,
    }
}

fn process(cwd: &Path) -> ObservedProcess {
    ObservedProcess {
        pid: 21,
        start_time: 21,
        parent_pid: 1,
        process_group: 21,
        foreground_group: 21,
        executable: crate::agent::codex::constants::EXECUTABLE.into(),
        argv: vec![crate::agent::codex::constants::EXECUTABLE.into()],
        environment: None,
        cwd: Some(cwd.into()),
        evidence_paths: Some(vec![cwd.join("evidence.jsonl")]),
    }
}

fn evidence(path: &Path, cwd: &Path, id: &str, state: AgentState) -> super::super::AgentEvidence {
    super::super::AgentEvidence {
        provider_session_id: id.into(),
        cwd: cwd.into(),
        path: path.into(),
        data_home: path.parent().unwrap().into(),
        state,
    }
}

#[test]
fn exit_transition_is_idempotent_with_and_without_a_binding() {
    let cwd = PathBuf::from("/workspace");
    let registry = AgentRegistry::from_adapters(Vec::new()).unwrap();
    let mut detector = AgentDetector::new(context(cwd.clone()), registry, None).unwrap();
    assert_eq!(detector.mark_exited(), DetectionResult::Unchanged);

    let mut exited = binding(AgentKind::Codex, Path::new("/evidence"), AgentState::Exited);
    exited.recovery_availability = AgentRecoveryAvailability::Unavailable;
    detector.binding = Some(exited);
    assert_eq!(detector.mark_exited(), DetectionResult::Unchanged);

    detector.binding.as_mut().unwrap().process_pid = Some(21);
    detector.binding.as_mut().unwrap().process_start_time = Some(21);
    assert!(matches!(
        detector.mark_exited(),
        DetectionResult::StatusChanged(_)
    ));

    detector.binding.as_mut().unwrap().state = AgentState::Running;
    detector.binding.as_mut().unwrap().process_pid = Some(21);
    detector.binding.as_mut().unwrap().process_start_time = Some(21);
    assert!(matches!(
        detector.mark_exited(),
        DetectionResult::StatusChanged(value)
            if value.state == AgentState::Exited && value.process_pid.is_none()
    ));
    assert_eq!(detector.mark_exited(), DetectionResult::Unchanged);
}

#[test]
fn candidate_transitions_preserve_identity_time_and_provider_state() {
    let directory = tempdir().unwrap();
    let cwd = directory.path();
    let first_path = cwd.join("first.jsonl");
    let second_path = cwd.join("second.jsonl");
    let registry = AgentRegistry::from_adapters(Vec::new()).unwrap();
    let mut detector = AgentDetector::new(context(cwd.into()), registry, None).unwrap();
    let observed = process(cwd);

    let changed = detector.apply_candidate(
        AgentKind::Codex,
        &observed,
        evidence(&first_path, cwd, "session", AgentState::Unknown),
    );
    let DetectionResult::BindingChanged(first) = changed else {
        panic!("first candidate must create a binding")
    };
    assert_eq!(first.state, AgentState::Unknown);
    assert_eq!(
        first.recovery_availability,
        crate::agent::AgentRecoveryAvailability::Unavailable
    );
    let detected_at = first.detected_at_ms;

    assert!(matches!(
        detector.apply_candidate(
            AgentKind::Codex,
            &observed,
            evidence(&first_path, cwd, "session", AgentState::Running),
        ),
        DetectionResult::StatusChanged(_)
    ));
    let status = detector.apply_candidate(
        AgentKind::Codex,
        &observed,
        evidence(&first_path, cwd, "session", AgentState::WaitingForInput),
    );
    assert!(matches!(
        status,
        DetectionResult::StatusChanged(value) if value.detected_at_ms == detected_at
    ));

    let identity = detector.apply_candidate(
        AgentKind::Codex,
        &observed,
        evidence(&second_path, cwd, "session", AgentState::WaitingForInput),
    );
    assert!(matches!(
        identity,
        DetectionResult::BindingChanged(value) if value.detected_at_ms == detected_at
    ));
}

struct InspectFailure;

impl super::super::AgentAdapter for InspectFailure {
    fn kind(&self) -> AgentKind {
        AgentKind::Codex
    }
    fn matches_process(&self, _: &ObservedProcess) -> bool {
        true
    }
    fn owns_evidence_path(&self, _: &Path, _: &DetectionContext) -> bool {
        true
    }
    fn evidence_files(&self, context: &DetectionContext) -> Result<Vec<PathBuf>> {
        Ok(vec![context.expected_cwd.join("evidence.jsonl")])
    }
    fn inspect_evidence(&self, _: &Path, _: &Path) -> Result<Option<super::super::AgentEvidence>> {
        Err(anyhow!("injected inspection failure"))
    }
    fn resume_plan(&self, _: &AgentBinding) -> super::super::ResumePlan {
        super::super::ResumePlan {
            executable: crate::agent::codex::constants::EXECUTABLE.into(),
            args: Vec::new(),
            environment: Default::default(),
        }
    }
    fn validate_binding(&self, _: &AgentBinding, _: &DetectionContext) -> Result<()> {
        Ok(())
    }
}

#[test]
fn inspection_errors_are_propagated_after_process_filters_pass() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    fs::create_dir(&cwd).unwrap();
    fs::write(cwd.join("evidence.jsonl"), b"{}\n").unwrap();
    let registry = AgentRegistry::from_adapters(vec![Box::new(InspectFailure)]).unwrap();
    let mut detector = AgentDetector::new(context(cwd.clone()), registry, None).unwrap();
    let error = detector.detect(&[process(&cwd)]).unwrap_err();
    assert!(error.to_string().contains("injected inspection failure"));
}
