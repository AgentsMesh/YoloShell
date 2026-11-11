use super::*;
use crate::agent::{claude, codex};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::Path;
use tempfile::tempdir;
use uuid::Uuid;

const PROCESS_PID: u32 = 307;
const PROCESS_START: u64 = 509;
const PROCESS_GROUP: i32 = 41;
const CODEX_SESSION_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
const EVIDENCE_DATE_DIRECTORY: &str = "2026/08/05";
const EVIDENCE_FILE: &str =
    "rollout-2026-08-05T12-00-00-550e8400-e29b-41d4-a716-446655440000.jsonl";

fn context(root: &Path) -> DetectionContext {
    DetectionContext::new(
        Uuid::new_v4(),
        root.join("project"),
        [
            (AgentKind::Codex, root.join("host-codex")),
            (AgentKind::ClaudeCode, root.join("host-claude")),
        ],
    )
}

fn process(cwd: &Path, environment: Option<BTreeMap<String, String>>) -> ObservedProcess {
    ObservedProcess {
        pid: PROCESS_PID,
        start_time: PROCESS_START,
        parent_pid: 1,
        process_group: PROCESS_GROUP,
        foreground_group: PROCESS_GROUP,
        executable: codex::constants::EXECUTABLE.into(),
        argv: vec![codex::constants::EXECUTABLE.into()],
        environment,
        cwd: Some(cwd.into()),
        evidence_paths: None,
    }
}

#[test]
fn process_environment_overrides_each_provider_without_cross_contamination() {
    let directory = tempdir().unwrap();
    let root = directory.path();
    let codex_home = root.join("process-codex");
    let claude_home = root.join("process-claude");
    std::fs::create_dir(&codex_home).unwrap();
    std::fs::create_dir(&claude_home).unwrap();
    let canonical_codex_home = codex_home.canonicalize().unwrap();
    let canonical_claude_home = claude_home.canonicalize().unwrap();
    let registry = yolo_agent_providers::builtin_agent_registry();
    let base = context(root);
    let observed = process(
        &base.expected_cwd,
        Some(
            [
                (
                    codex::constants::DATA_HOME_ENV.to_owned(),
                    codex_home.display().to_string(),
                ),
                (
                    claude::constants::DATA_HOME_ENV.to_owned(),
                    claude_home.display().to_string(),
                ),
            ]
            .into(),
        ),
    );
    let codex = base.for_process(registry.adapter(&AgentKind::Codex).unwrap(), &observed);
    let claude = base.for_process(registry.adapter(&AgentKind::ClaudeCode).unwrap(), &observed);
    assert_eq!(
        codex.data_home(&AgentKind::Codex),
        Some(canonical_codex_home.as_path())
    );
    assert_eq!(
        codex.data_home(&AgentKind::ClaudeCode),
        base.data_home(&AgentKind::ClaudeCode)
    );
    assert_eq!(
        claude.data_home(&AgentKind::ClaudeCode),
        Some(canonical_claude_home.as_path())
    );
}

#[test]
fn readable_missing_or_invalid_provider_home_never_falls_back_to_host_context() {
    let directory = tempdir().unwrap();
    std::fs::create_dir(directory.path().join("root")).unwrap();
    std::fs::create_dir(directory.path().join("escape")).unwrap();
    let base = context(directory.path());
    let registry = yolo_agent_providers::builtin_agent_registry();
    let adapter = registry.adapter(&AgentKind::Codex).unwrap();
    for variables in [
        BTreeMap::new(),
        [(codex::constants::DATA_HOME_ENV.to_owned(), ".".to_owned())].into(),
        [(
            codex::constants::DATA_HOME_ENV.to_owned(),
            directory
                .path()
                .join("root/../escape")
                .display()
                .to_string(),
        )]
        .into(),
        [(
            codex::constants::DATA_HOME_ENV.to_owned(),
            directory.path().join("missing").display().to_string(),
        )]
        .into(),
    ] {
        let observed = process(&base.expected_cwd, Some(variables));
        assert!(
            base.for_process(adapter, &observed)
                .data_home(&AgentKind::Codex)
                .is_none()
        );
    }
    let unavailable = process(&base.expected_cwd, None);
    assert_eq!(
        base.for_process(adapter, &unavailable)
            .data_home(&AgentKind::Codex),
        base.data_home(&AgentKind::Codex)
    );
}

#[test]
fn detector_discovers_codex_evidence_from_the_agent_process_home() {
    let directory = tempdir().unwrap();
    let project = directory.path().join("project");
    let process_home = directory.path().join("process-codex");
    let evidence = process_home
        .join(codex::constants::EVIDENCE_DIR)
        .join(EVIDENCE_DATE_DIRECTORY)
        .join(EVIDENCE_FILE);
    std::fs::create_dir_all(&project).unwrap();
    std::fs::create_dir_all(evidence.parent().unwrap()).unwrap();
    let mut detector =
        AgentDetector::new(context(directory.path()), yolo_agent_providers::builtin_agent_registry(), None).unwrap();
    let record = json!({
        (codex::constants::RECORD_TYPE_KEY): codex::constants::SESSION_META_TYPE,
        (codex::constants::PAYLOAD_KEY): {
            (codex::constants::SESSION_ID_KEY): CODEX_SESSION_ID,
            (codex::constants::CWD_KEY): project,
            (codex::constants::SOURCE_KEY): codex::constants::CLI_SOURCE,
        },
    });
    std::fs::write(
        &evidence,
        format!("{record}{}", constants::COMMAND_LINE_ENDING),
    )
    .unwrap();
    let environment = [(
        codex::constants::DATA_HOME_ENV.to_owned(),
        process_home.display().to_string(),
    )]
    .into();
    let mut observed = process(&project, Some(environment));
    observed.evidence_paths = Some(vec![evidence.canonicalize().unwrap()]);
    let result = detector.detect(&[observed]).unwrap();
    assert!(matches!(
        result,
        DetectionResult::BindingChanged(binding)
            if binding.data_home == process_home.canonicalize().unwrap().display().to_string()
                && binding.process_start_time == Some(PROCESS_START)
    ));
}
