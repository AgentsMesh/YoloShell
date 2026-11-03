use crate::agent::{AgentAdapter, DetectionContext};
use crate::grok::GrokAdapter;
use crate::grok::constants;
use std::path::{Path, PathBuf};
use uuid::Uuid;

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, String) {
    let root = tempfile::tempdir().unwrap();
    let cwd = std::fs::canonicalize(root.path()).unwrap();
    let data_home = cwd.join(".grok");
    let session = Uuid::new_v4().to_string();
    let directory = data_home.join("sessions").join("project").join(&session);
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("updates.jsonl");
    std::fs::write(
        &path,
        format!(
            "{{\"timestamp\":1,\"method\":\"_x.ai/session/update\",\"params\":{{\"sessionId\":\"{session}\"}}}}\n"
        ),
    )
    .unwrap();
    (root, cwd, data_home, session)
}

fn binding(cwd: &Path, data_home: &Path, session: &str) -> crate::AgentBinding {
    crate::AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: GrokAdapter.kind(),
        provider_session_id: session.to_owned(),
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
    }
}

#[test]
fn session_directory_identifies_the_evidence() {
    let (_root, cwd, data_home, session) = fixture();
    let path = data_home
        .join("sessions")
        .join("project")
        .join(&session)
        .join("updates.jsonl");
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(GrokAdapter.kind(), data_home.clone())],
    );
    let evidence = GrokAdapter
        .inspect_evidence_in_context(&path, &cwd, &context)
        .unwrap()
        .unwrap();
    assert_eq!(evidence.provider_session_id, session);
    assert_eq!(evidence.cwd, cwd);
    assert_eq!(evidence.data_home, data_home);
    assert!(!GrokAdapter.requires_evidence_cwd_match());
}

#[test]
fn a_file_outside_a_session_directory_is_not_evidence() {
    let (_root, cwd, data_home, _session) = fixture();
    let other = data_home.join("sessions").join("loose.jsonl");
    std::fs::write(&other, b"{}\n").unwrap();
    let context = DetectionContext::new(
        Uuid::new_v4(),
        cwd.clone(),
        [(GrokAdapter.kind(), data_home.clone())],
    );
    assert!(
        GrokAdapter
            .inspect_evidence_in_context(&other, &cwd, &context)
            .unwrap()
            .is_none()
    );
}

#[test]
fn resume_plan_uses_the_exact_session_and_home() {
    let (_root, cwd, data_home, session) = fixture();
    let plan = GrokAdapter.resume_plan(&binding(&cwd, &data_home, &session));
    assert_eq!(plan.executable, "grok");
    assert_eq!(plan.args, ["-r", session.as_str()]);
    assert_eq!(
        plan.environment.get(constants::DATA_HOME_ENV),
        Some(&data_home.to_string_lossy().into_owned())
    );
}

#[test]
fn executable_and_exact_session_arguments_match() {
    use crate::agent::ObservedProcess;
    let mut process = ObservedProcess {
        pid: 10,
        start_time: 10,
        parent_pid: 1,
        process_group: 10,
        foreground_group: 10,
        executable: "/home/dev/.local/bin/grok".into(),
        argv: vec!["grok".into(), "-r".into()],
        environment: None,
        cwd: Some("/tmp".into()),
        evidence_paths: None,
    };
    assert!(GrokAdapter.matches_process(&process));
    process.executable = "/home/dev/.grok/downloads/grok-linux-aarch64".into();
    assert!(GrokAdapter.matches_process(&process));
    let exact = Uuid::new_v4().to_string();
    process.argv.push(exact.clone());
    assert_eq!(GrokAdapter.process_session_id(&process), Some(exact.clone()));
    assert_eq!(GrokAdapter.process_resume_session_id(&process), Some(exact.clone()));
    // The inline form names the same exact session without a value argument.
    process.argv = vec![
        "grok".into(),
        format!(
            "{}{}{exact}",
            constants::RESUME_ARGUMENT,
            constants::ARGUMENT_VALUE_SEPARATOR
        ),
    ];
    assert_eq!(GrokAdapter.process_session_id(&process), Some(exact.clone()));
    assert_eq!(GrokAdapter.process_resume_session_id(&process), Some(exact));
    let _ = Path::new("/");
}

#[test]
fn a_resume_title_is_not_an_exact_identity() {
    use crate::agent::ObservedProcess;
    let process = ObservedProcess {
        pid: 10,
        start_time: 10,
        parent_pid: 1,
        process_group: 10,
        foreground_group: 10,
        executable: "grok".into(),
        argv: vec![
            "grok".into(),
            constants::RESUME_ARGUMENT.into(),
            "fix login bug".into(),
        ],
        environment: None,
        cwd: None,
        evidence_paths: None,
    };
    assert!(GrokAdapter.process_session_arguments_valid(&process));
    assert_eq!(GrokAdapter.process_session_id(&process), None);
    assert_eq!(GrokAdapter.process_resume_session_id(&process), None);
}
