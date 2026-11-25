use super::*;
use crate::agent::{AgentKind, AgentRecoveryAvailability, AgentState};

#[cfg(feature = "yolo-unit-tests")]
#[test]
fn availability_only_invalidation_publishes_a_status_event() {
    let before = binding();
    let mut after = before.clone();
    after.recovery_availability = AgentRecoveryAvailability::Unavailable;

    let commit = if_changed(Some(&before), Some(after.clone())).unwrap();

    assert_eq!(commit.event_type, event::AGENT_STATUS_CHANGED);
    assert_eq!(commit.binding, after);
}

fn binding() -> AgentBinding {
    let data_home = std::env::temp_dir();
    AgentBinding {
        schema_version: crate::agent::constants::BINDING_SCHEMA_VERSION,
        terminal_id: uuid::Uuid::new_v4(),
        kind: AgentKind::ClaudeCode,
        provider_session_id: uuid::Uuid::new_v4().hyphenated().to_string(),
        executable: crate::agent::claude::constants::EXECUTABLE.to_owned(),
        cwd: data_home.to_string_lossy().into_owned(),
        evidence_path: data_home
            .join(crate::agent::constants::BINDING_FILE_NAME)
            .to_string_lossy()
            .into_owned(),
        data_home: data_home.to_string_lossy().into_owned(),
        process_pid: None,
        process_start_time: None,
        state: AgentState::Exited,
        recovery_availability: AgentRecoveryAvailability::Available,
        detected_at_ms: i64::default(),
        updated_at_ms: i64::default(),
    }
}
