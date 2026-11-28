use crate::fixtures::{fields, round_trip};
use yolo_hub_agent_protocol::*;

const PROJECT_DIRECTORY: &str = "/remote/work/项目 with spaces";
const ROOT_DIRECTORY: &str = "/";
const PATH_CHARACTER: &str = "x";
const UNICODE_CHARACTER: &str = "项";

#[test]
fn directory_is_bounded_origin_metadata_with_exact_wire_round_trip() {
    let maximum = format!(
        "{ROOT_DIRECTORY}{}",
        PATH_CHARACTER.repeat(MAX_AGENT_WORKING_DIRECTORY_BYTES - ROOT_DIRECTORY.len(),)
    );
    for path in [ROOT_DIRECTORY, PROJECT_DIRECTORY, maximum.as_str()] {
        let directory = AgentWorkingDirectory::new(path.into()).unwrap();
        assert_eq!(directory.as_str(), path);
        assert_eq!(round_trip(&directory), directory);
        assert_eq!(serde_json::to_value(&directory).unwrap(), path);
        assert_eq!(String::from(directory), path);
    }
}

#[test]
fn invalid_wire_paths_do_not_become_directory_observations() {
    let too_long = format!(
        "{ROOT_DIRECTORY}{}",
        PATH_CHARACTER.repeat(MAX_AGENT_WORKING_DIRECTORY_BYTES)
    );
    let unicode_too_long = format!(
        "{ROOT_DIRECTORY}{}",
        UNICODE_CHARACTER.repeat(
            MAX_AGENT_WORKING_DIRECTORY_BYTES / UNICODE_CHARACTER.len() + ROOT_DIRECTORY.len(),
        )
    );
    for path in [
        "",
        "relative",
        "~/project",
        "/work\0other",
        "/work\nother",
        "/work\u{1b}[2J",
        "/work\u{7f}",
        too_long.as_str(),
        unicode_too_long.as_str(),
    ] {
        assert_eq!(
            AgentWorkingDirectory::new(path.into()),
            Err(ProtocolError::InvalidWorkingDirectory)
        );
        assert!(serde_json::from_value::<AgentWorkingDirectory>(path.into()).is_err());
    }
    for wire in [
        serde_json::Value::Null,
        true.into(),
        serde_json::json!({"path": PROJECT_DIRECTORY}),
    ] {
        assert!(serde_json::from_value::<AgentWorkingDirectory>(wire).is_err());
    }
}

#[test]
fn missing_or_null_observation_is_unknown_without_inventing_a_shell_directory() {
    let snapshot = AgentSnapshot::new(fields()).unwrap();
    let mut wire = serde_json::to_value(&snapshot).unwrap();
    assert!(wire.get(AGENT_WORKING_DIRECTORY_FIELD).is_none());
    assert!(
        serde_json::from_value::<AgentSnapshot>(wire.clone())
            .unwrap()
            .fields()
            .working_directory
            .is_none()
    );
    wire[AGENT_WORKING_DIRECTORY_FIELD] = serde_json::Value::Null;
    assert_eq!(
        serde_json::from_value::<AgentSnapshot>(wire.clone()).unwrap(),
        snapshot
    );
    wire[AGENT_WORKING_DIRECTORY_FIELD] = PROJECT_DIRECTORY.into();
    let observed = serde_json::from_value::<AgentSnapshot>(wire.clone()).unwrap();
    assert_eq!(
        observed
            .fields()
            .working_directory
            .as_ref()
            .unwrap()
            .as_str(),
        PROJECT_DIRECTORY
    );
    assert_eq!(observed.cursor(), snapshot.cursor());
    wire[AGENT_WORKING_DIRECTORY_FIELD] = "relative".into();
    assert!(serde_json::from_value::<AgentSnapshot>(wire).is_err());
}

#[test]
fn cached_directory_keeps_the_original_execution_and_explicit_staleness() {
    let mut fields = fields();
    let original = fields.cursor;
    fields.working_directory = Some(AgentWorkingDirectory::new(PROJECT_DIRECTORY.into()).unwrap());
    fields.freshness = Freshness::Stale;
    fields.reachability = Reachability::Unreachable;
    let snapshot = AgentSnapshot::new(fields).unwrap();
    let decoded = round_trip(&snapshot);
    assert_eq!(decoded.cursor(), original);
    assert_eq!(
        decoded.fields().working_directory,
        snapshot.fields().working_directory
    );
    assert_eq!(decoded.fields().freshness, Freshness::Stale);
    assert_eq!(decoded.fields().reachability, Reachability::Unreachable);
}
