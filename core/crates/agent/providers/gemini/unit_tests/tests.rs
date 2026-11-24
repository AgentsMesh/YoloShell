use crate::agent::{AgentAdapter, DetectionContext};
use crate::gemini::GeminiAdapter;
use crate::gemini::constants;
use std::path::Path;
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, std::path::PathBuf, std::path::PathBuf, String) {
    let root = tempfile::tempdir().unwrap();
    let cwd = std::fs::canonicalize(root.path()).unwrap();
    let data_home = cwd.join(".gemini");
    let chats = data_home.join("tmp").join("project").join("chats");
    std::fs::create_dir_all(&chats).unwrap();
    let session = Uuid::new_v4().to_string();
    let short = &session[..8];
    let path = chats.join(format!("session-2026-10-29T03-05-{short}.jsonl"));
    std::fs::write(
        &path,
        format!("{{\"sessionId\":\"{session}\",\"projectHash\":\"hash\",\"kind\":\"main\"}}\n"),
    )
    .unwrap();
    std::fs::write(
        &path,
        format!(
            "{{\"sessionId\":\"{session}\",\"projectHash\":\"hash\",\"kind\":\"main\"}}\n{{\"type\":\"user\",\"content\":[{{\"text\":\"hello\"}}]}}\n{{\"$set\":{{\"lastUpdated\":\"now\"}}}}\n"
        ),
    )
    .unwrap();
    (root, cwd, data_home, session)
}

#[test]
fn transcript_header_identifies_the_session() {
    let (_root, cwd, data_home, session) = fixture();
    let path = std::fs::read_dir(data_home.join("tmp").join("project").join("chats"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(GeminiAdapter.kind(), data_home.clone())],
    );
    let evidence = GeminiAdapter
        .inspect_evidence_in_context(&path, &cwd, &context)
        .unwrap()
        .unwrap();
    assert_eq!(evidence.provider_session_id, session);
    assert_eq!(evidence.cwd, cwd);
    assert_eq!(evidence.data_home, data_home);
    assert!(!GeminiAdapter.requires_evidence_cwd_match());
}

#[test]
fn a_foreign_header_is_not_this_agents_evidence() {
    let (_root, cwd, data_home, _session) = fixture();
    let path = std::fs::read_dir(data_home.join("tmp").join("project").join("chats"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut document = std::fs::read(&path).unwrap();
    document.extend_from_slice(b"");
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(GeminiAdapter.kind(), data_home.clone())],
    );
    let other = cwd.join("other.jsonl");
    std::fs::write(&other, b"{\"sessionId\":\"not-a-session\"}\n").unwrap();
    assert!(
        GeminiAdapter
            .inspect_evidence_in_context(&other, &cwd, &context)
            .unwrap()
            .is_none()
    );
}

#[test]
fn resume_plan_uses_the_exact_session_and_isolated_home() {
    let (_root, cwd, data_home, session) = fixture();
    let binding = crate::AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: GeminiAdapter.kind(),
        provider_session_id: session.clone(),
        executable: constants::EXECUTABLE.into(),
        cwd: cwd.to_string_lossy().into_owned(),
        data_home: data_home.to_string_lossy().into_owned(),
        evidence_path: String::new(),
        process_pid: Some(1),
        process_start_time: Some(1),
        state: crate::AgentState::WaitingForInput,
        recovery_availability: crate::AgentRecoveryAvailability::Available,
        detected_at_ms: 1,
        updated_at_ms: 1,
    };
    let plan = GeminiAdapter.resume_plan(&binding);
    assert_eq!(plan.executable, "gemini");
    assert_eq!(
        plan.args,
        [constants::RESUME_ARGUMENT, session.as_str()]
    );
    assert_eq!(
        plan.environment.get(constants::DATA_HOME_ENV),
        Some(&cwd.to_string_lossy().into_owned())
    );
}

#[test]
fn node_entrypoint_and_exact_arguments_match() {
    use crate::agent::ObservedProcess;
    let _ = Path::new("/");
    let mut process = ObservedProcess {
        pid: 10,
        start_time: 10,
        parent_pid: 1,
        process_group: 10,
        foreground_group: 10,
        executable: "/usr/bin/node".into(),
        argv: vec![
            "node".into(),
            "/opt/gemini/node_modules/.bin/gemini".into(),
            constants::RESUME_ARGUMENT.into(),
        ],
        environment: None,
        cwd: Some("/tmp".into()),
        evidence_paths: None,
    };
    assert!(GeminiAdapter.matches_process(&process));
    let exact = Uuid::new_v4().to_string();
    process.argv.push(exact.clone());
    assert_eq!(GeminiAdapter.process_session_id(&process), Some(exact));
}
