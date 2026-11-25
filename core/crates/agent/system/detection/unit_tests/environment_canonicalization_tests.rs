use super::*;
use yolo_claude::claude;
use yolo_codex::codex;
use tempfile::tempdir;
use uuid::Uuid;

const EXISTING_RELATIVE_HOME: &str = ".";
const PARENT_DIRECTORY: &str = "parent";
const PARENT_COMPONENT: &str = "..";
const TARGET_DIRECTORY: &str = "target";

#[test]
fn host_context_canonicalizes_existing_homes_for_every_provider() {
    let directory = tempdir().unwrap();
    let codex_home = directory.path().join("codex");
    let claude_home = directory.path().join("claude");
    std::fs::create_dir(&codex_home).unwrap();
    std::fs::create_dir(&claude_home).unwrap();
    let values = [
        (
            codex::constants::DATA_HOME_ENV.into(),
            codex_home.clone().into_os_string(),
        ),
        (
            claude::constants::DATA_HOME_ENV.into(),
            claude_home.clone().into_os_string(),
        ),
    ];
    let environment = DetectionEnvironment::from_values(values, &Default::default());
    let context = yolo_agent_providers::builtin_agent_registry()
        .detection_context_with_environment(Uuid::new_v4(), directory.path().into(), &environment)
        .unwrap();
    let canonical_codex_home = codex_home.canonicalize().unwrap();
    let canonical_claude_home = claude_home.canonicalize().unwrap();
    assert_eq!(
        context.data_home(&AgentKind::Codex),
        Some(canonical_codex_home.as_path())
    );
    assert_eq!(
        context.data_home(&AgentKind::ClaudeCode),
        Some(canonical_claude_home.as_path())
    );
}

#[test]
fn host_context_rejects_existing_ambiguous_provider_paths() {
    let directory = tempdir().unwrap();
    std::fs::create_dir(directory.path().join(PARENT_DIRECTORY)).unwrap();
    std::fs::create_dir(directory.path().join(TARGET_DIRECTORY)).unwrap();
    let parent_path = directory
        .path()
        .join(PARENT_DIRECTORY)
        .join(PARENT_COMPONENT)
        .join(TARGET_DIRECTORY);
    for value in [EXISTING_RELATIVE_HOME.into(), parent_path.into_os_string()] {
        let environment = DetectionEnvironment::from_values(
            [(codex::constants::DATA_HOME_ENV.into(), value)],
            &Default::default(),
        );
        assert!(
            yolo_agent_providers::builtin_agent_registry()
                .detection_context_with_environment(
                    Uuid::new_v4(),
                    directory.path().into(),
                    &environment,
                )
                .is_none()
        );
    }
}
