use super::super::*;
use super::fixture;
use crate::agent::AgentRecoveryAvailability;
use std::fs;
use tempfile::tempdir;
use uuid::Uuid;

#[test]
fn real_schema_fixture_drives_resume_and_main_transcript_filtering() {
    let directory = tempdir().unwrap();
    let cwd = PathBuf::from(fixture::CWD);
    let home = directory.path().join(fixture::DATA_HOME_DIRECTORY);
    let root = home.join(constants::EVIDENCE_DIR);
    let project = discovery::project_directory(&root, &cwd);
    fs::create_dir_all(&project).unwrap();
    let file_name = format!(
        "{}.{}",
        fixture::SESSION_ID,
        crate::agent::constants::JSONL_EXTENSION
    );
    let evidence = project.join(&file_name);
    let subagent = project.join(format!(
        "{}{file_name}",
        constants::SUBAGENT_TRANSCRIPT_PREFIX
    ));
    fs::write(&evidence, fixture::FIXTURE_BYTES).unwrap();
    fs::write(&subagent, fixture::FIXTURE_BYTES).unwrap();

    let adapter = ClaudeAdapter;
    let found = adapter.inspect_evidence(&evidence, &cwd).unwrap().unwrap();
    assert_eq!(found.provider_session_id, fixture::SESSION_ID);
    assert_eq!(found.cwd, cwd);
    assert_eq!(found.path, evidence);
    assert_eq!(found.data_home, home);
    assert_eq!(found.state, AgentState::Unknown);
    assert!(adapter.inspect_evidence(&subagent, &cwd).unwrap().is_none());
    assert_eq!(
        discovery::evidence_files(&root, &cwd).unwrap(),
        [evidence.clone()]
    );

    let binding = AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: Uuid::new_v4(),
        kind: AgentKind::ClaudeCode,
        provider_session_id: found.provider_session_id,
        executable: constants::EXECUTABLE.to_owned(),
        cwd: found.cwd.display().to_string(),
        evidence_path: found.path.display().to_string(),
        data_home: found.data_home.display().to_string(),
        process_pid: None,
        process_start_time: None,
        state: found.state,
        recovery_availability: AgentRecoveryAvailability::Unavailable,
        detected_at_ms: i64::default(),
        updated_at_ms: i64::default(),
    };
    assert_eq!(
        adapter.resume_plan(&binding),
        ResumePlan {
            executable: constants::EXECUTABLE.to_owned(),
            args: vec![
                constants::RESUME_ARGUMENT.to_owned(),
                fixture::SESSION_ID.to_owned(),
            ],
            environment: [(
                constants::DATA_HOME_ENV.to_owned(),
                home.display().to_string(),
            )]
            .into(),
        }
    );
}
