use super::branch_support_tests::*;
use super::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn process_filters_require_a_working_directory_and_foreground_state() {
    let directory = tempdir().unwrap();
    let cwd = directory.path().join("project");
    fs::create_dir(&cwd).unwrap();
    let path = directory.path().join("evidence.jsonl");
    fs::write(&path, b"{}\n").unwrap();
    let mut detector = AgentDetector::new(context(&cwd), registry(path), None).unwrap();

    assert_eq!(
        detector.detect(&[process(None, PROCESS_PID)]).unwrap(),
        DetectionResult::Unchanged
    );
    let mut stopped = process(Some(&cwd), PROCESS_PID);
    stopped.foreground_group = 0;
    assert_eq!(
        detector.detect(&[stopped]).unwrap(),
        DetectionResult::Unchanged
    );
}

#[test]
fn stable_path_requires_matching_kind_process_or_resume() {
    let directory = tempdir().unwrap();
    let cwd = directory.path();
    let discovered = cwd.join("discovered.jsonl");
    let stable = cwd.join("stable.jsonl");
    let existing = binding(AgentKind::ClaudeCode, &stable);
    let mut detector =
        AgentDetector::new(context(cwd), registry(discovered.clone()), Some(existing)).unwrap();
    let observed = process(Some(cwd), PROCESS_PID);
    assert_eq!(
        detector
            .paths_for_process(&AgentKind::Codex, &observed)
            .unwrap(),
        std::slice::from_ref(&discovered)
    );

    detector.binding = Some(binding(AgentKind::Codex, &stable));
    assert_eq!(
        detector
            .paths_for_process(&AgentKind::Codex, &observed)
            .unwrap(),
        vec![discovered.clone(), stable.clone()]
    );
    let mut contaminated = observed.clone();
    contaminated.evidence_paths = Some(vec![discovered.clone()]);
    assert_eq!(
        detector
            .paths_for_process(&AgentKind::Codex, &contaminated)
            .unwrap(),
        std::slice::from_ref(&stable)
    );
    detector.binding.as_mut().unwrap().state = AgentState::Exited;
    detector.binding.as_mut().unwrap().process_pid = None;
    detector.binding.as_mut().unwrap().process_start_time = None;
    let mut resumed = observed;
    resumed.argv.extend([
        crate::agent::codex::constants::RESUME_ARGUMENT.into(),
        BOUND_SESSION_ID.into(),
    ]);
    assert_eq!(
        detector
            .paths_for_process(&AgentKind::Codex, &resumed)
            .unwrap(),
        vec![discovered.clone(), stable.clone()]
    );
    assert_eq!(
        detector
            .paths_for_process(&AgentKind::Codex, &process(Some(cwd), OTHER_PROCESS_PID))
            .unwrap(),
        [discovered]
    );
}

#[test]
fn kind_session_and_pid_changes_are_classified_separately() {
    let directory = tempdir().unwrap();
    let cwd = directory.path();
    let path = cwd.join("evidence.jsonl");
    let registry = AgentRegistry::from_adapters(Vec::new()).unwrap();
    let existing = binding(AgentKind::Codex, &path);
    let mut detector = AgentDetector::new(context(cwd), registry, Some(existing)).unwrap();

    assert!(matches!(
        detector.apply_candidate(
            AgentKind::ClaudeCode,
            &process(Some(cwd), PROCESS_PID),
            evidence(&path, cwd, BOUND_SESSION_ID),
        ),
        DetectionResult::BindingChanged(_)
    ));
    assert!(matches!(
        detector.apply_candidate(
            AgentKind::ClaudeCode,
            &process(Some(cwd), PROCESS_PID),
            evidence(&path, cwd, DIFFERENT_SESSION_ID),
        ),
        DetectionResult::BindingChanged(_)
    ));
    assert!(matches!(
        detector.apply_candidate(
            AgentKind::ClaudeCode,
            &process(Some(cwd), CHANGED_PROCESS_PID),
            evidence(&path, cwd, DIFFERENT_SESSION_ID),
        ),
        DetectionResult::StatusChanged(_)
    ));
}
