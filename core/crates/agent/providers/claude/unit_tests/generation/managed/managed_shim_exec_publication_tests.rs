use super::super::SessionStartSource;
use super::managed_shim_exec_publication_test_support::{Case, path_text};
use super::observed_unmanaged_publication_test_support::{
    ABSOLUTE_NODE, CLAUDE_ENTRYPOINT, FOREIGN_ENTRYPOINT,
};
use crate::agent::{AgentKind, AgentRecoveryAvailability, AgentState};
use std::path::PathBuf;

const FOREIGN_CWD: &str = "/foreign/project";
const FOREIGN_EXECUTABLE: &str = "/usr/bin/not-claude";
const RELATIVE_SHIM: &str = "bin/claude";
const FOREIGN_EVIDENCE_SUFFIX: &str = ".foreign";
const FOREIGN_DATA_HOME_SUFFIX: &str = "-foreign";
const IDENTITY_INCREMENT: u64 = 1;
const INVALID_TIMESTAMP: i64 = -1;

#[test]
fn exact_shim_exec_transition_accepts_supported_successors_and_states() {
    for state in [AgentState::Running, AgentState::WaitingForInput] {
        assert!(Case::native(state).accepts());
    }
    let node = Case::new(
        AgentState::Running,
        ABSOLUTE_NODE,
        vec![ABSOLUTE_NODE.to_owned(), CLAUDE_ENTRYPOINT.to_owned()],
    );
    assert!(node.accepts());
    assert!(Case::resumed(AgentState::Running, SessionStartSource::Resume).accepts());
    assert!(Case::resumed(AgentState::Running, SessionStartSource::Fork).accepts());
}

#[test]
fn predecessor_identity_and_subject_must_match_exactly() {
    for mutate in binding_mutations() {
        let case = Case::native(AgentState::Running);
        let mut predecessor = case.predecessor.clone();
        mutate(&case, &mut predecessor);
        assert!(!case.accepts_at(
            &case.authority,
            &case.successor,
            &predecessor,
            &case.authority.handshake_boot_id,
            case.authority.handshake_deadline_ns - IDENTITY_INCREMENT,
        ));
    }
}

#[test]
fn successor_must_be_the_same_live_non_shim_claude_process() {
    for mutate in successor_mutations() {
        let case = Case::native(AgentState::Running);
        let mut successor = case.successor.clone();
        mutate(&case, &mut successor);
        assert!(!case.accepts_at(
            &case.authority,
            &successor,
            &case.predecessor,
            &case.authority.handshake_boot_id,
            case.authority.handshake_deadline_ns - IDENTITY_INCREMENT,
        ));
    }
}

type BindingMutation = fn(&Case, &mut crate::agent::AgentBinding);

fn binding_mutations() -> Vec<BindingMutation> {
    vec![
        |_, value| value.schema_version += IDENTITY_INCREMENT as u32,
        |_, value| value.terminal_id = uuid::Uuid::new_v4(),
        |_, value| value.kind = AgentKind::Codex,
        |_, value| value.provider_session_id = uuid::Uuid::new_v4().hyphenated().to_string(),
        |case, value| value.executable = path_text(&case.foreign_shim()),
        |_, value| value.executable = RELATIVE_SHIM.to_owned(),
        |_, value| value.cwd = FOREIGN_CWD.to_owned(),
        |_, value| value.evidence_path.push_str(FOREIGN_EVIDENCE_SUFFIX),
        |_, value| value.data_home.push_str(FOREIGN_DATA_HOME_SUFFIX),
        |_, value| value.process_pid = value.process_pid.map(|pid| pid + IDENTITY_INCREMENT as u32),
        |_, value| {
            value.process_start_time = value
                .process_start_time
                .map(|start| start + IDENTITY_INCREMENT)
        },
        |_, value| value.state = AgentState::Running,
        |_, value| value.recovery_availability = AgentRecoveryAvailability::UnmanagedLaunch,
        |_, value| value.recovery_availability = AgentRecoveryAvailability::Available,
        |_, value| value.detected_at_ms = INVALID_TIMESTAMP,
        |_, value| value.updated_at_ms = value.detected_at_ms - IDENTITY_INCREMENT as i64,
    ]
}

type SuccessorMutation = fn(&Case, &mut crate::agent::ObservedProcess);

fn successor_mutations() -> Vec<SuccessorMutation> {
    vec![
        |_, value| value.pid += IDENTITY_INCREMENT as u32,
        |_, value| value.start_time += IDENTITY_INCREMENT,
        |_, value| value.cwd = Some(PathBuf::from(FOREIGN_CWD)),
        |_, value| value.process_group += IDENTITY_INCREMENT as i32,
        |_, value| value.foreground_group += IDENTITY_INCREMENT as i32,
        |case, value| value.executable = path_text(&case.shim),
        |_, value| value.executable = FOREIGN_EXECUTABLE.to_owned(),
        |_, value| {
            value.executable = ABSOLUTE_NODE.to_owned();
            value.argv = vec![ABSOLUTE_NODE.to_owned(), FOREIGN_ENTRYPOINT.to_owned()];
        },
    ]
}
