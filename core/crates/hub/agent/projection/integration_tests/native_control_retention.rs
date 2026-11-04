use super::{
    constants::*,
    control_support::*,
    fixtures::{directory, open},
    view::view,
};
use yolo_hub_agent_protocol::{Capability, Freshness};
use yolo_terminal_model::RecoveryState;

#[test]
fn invalidating_a_pending_interaction_clears_answerability_and_keeps_the_record_valid() {
    let (_root, path, hub) = directory();
    let view = view();
    let observed = observation_with_interaction(&view, IDENTITY_STEP, uuid::Uuid::new_v4());
    let mut projection = open(&path, hub);
    let current = projection
        .project_with_observation(&view, Some(&observed))
        .unwrap()
        .unwrap();
    assert!(current.fields().interaction.is_some());
    projection.mark_unreachable().unwrap();
    let stale = projection
        .get(current.cursor().target().address)
        .unwrap()
        .unwrap();
    assert!(stale.fields().interaction.is_none());
    assert_eq!(
        stale.fields().capabilities.respond_interaction,
        Capability::Unsupported {}
    );
    assert_eq!(stale.fields().freshness, Freshness::Stale);
}

#[test]
fn stale_restart_and_absent_source_keep_only_receipt_lookup_witness() {
    let (_root, path, hub) = directory();
    let view = view();
    let observed = observation(&view, IDENTITY_STEP);
    let mut projection = open(&path, hub);
    let current = projection
        .project_with_observation(&view, Some(&observed))
        .unwrap()
        .unwrap();
    projection.mark_unreachable().unwrap();
    let stale = projection
        .get(current.cursor().target().address)
        .unwrap()
        .unwrap();
    assert_eq!(stale.fields().freshness, Freshness::Stale);
    assert!(matches!(
        stale.fields().capabilities.interrupt,
        Capability::Unavailable { .. }
    ));
    assert!(projection.control_witness(&request(&current)).is_err());
    assert_eq!(
        projection
            .retained_control_witness(current.cursor().target())
            .unwrap(),
        Some(observed.witness().clone())
    );
    drop(projection);
    let mut reopened = open(&path, hub);
    assert_eq!(
        reopened
            .retained_control_witness(current.cursor().target())
            .unwrap(),
        Some(observed.witness().clone())
    );
    let missing = reopened.project(&view).unwrap().unwrap();
    assert!(matches!(
        missing.fields().capabilities.interrupt,
        Capability::Unavailable { .. }
    ));
    assert!(reopened.control_witness(&request(&missing)).is_err());
    let restored = reopened
        .project_with_observation(&view, Some(&observed))
        .unwrap()
        .unwrap();
    assert_eq!(restored.cursor().target(), current.cursor().target());
    assert!(
        reopened
            .control_witness(&request(&restored))
            .unwrap()
            .is_some()
    );
}

#[test]
fn resumed_logical_agent_retains_previous_execution_witness_for_receipts_only() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let mut view = view();
    let original = observation(&view, IDENTITY_STEP);
    let before = projection
        .project_with_observation(&view, Some(&original))
        .unwrap()
        .unwrap();
    view.runtime.as_mut().unwrap().runtime_epoch = uuid::Uuid::new_v4();
    view.runtime.as_mut().unwrap().worker_id = uuid::Uuid::new_v4();
    view.manifest.recovery_state = RecoveryState::Resumed;
    let binding = view.agent.as_mut().unwrap();
    binding.process_pid = binding
        .process_pid
        .map(|pid| pid + u32::try_from(IDENTITY_STEP).unwrap());
    binding.process_start_time = binding
        .process_start_time
        .map(|start| start + IDENTITY_STEP);
    binding.updated_at_ms += i64::try_from(IDENTITY_STEP).unwrap();
    let observed = observation(&view, IDENTITY_STEP + IDENTITY_STEP);
    let current = projection
        .project_with_observation(&view, Some(&observed))
        .unwrap()
        .unwrap();
    assert_eq!(
        current.cursor().target().address,
        before.cursor().target().address
    );
    assert_ne!(
        current.cursor().target().execution_id,
        before.cursor().target().execution_id
    );
    assert_eq!(
        projection
            .retained_control_witness(before.cursor().target())
            .unwrap(),
        Some(original.witness().clone())
    );
    assert!(projection.control_witness(&request(&before)).is_err());
    assert_eq!(
        projection.control_witness(&request(&current)).unwrap(),
        Some(observed.witness().clone())
    );
    drop(projection);
    let reopened = open(&path, hub);
    assert_eq!(
        reopened
            .retained_control_witness(before.cursor().target())
            .unwrap(),
        Some(original.witness().clone())
    );
}

#[test]
fn an_old_record_without_optional_control_fields_keeps_existing_public_identity() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let view = view();
    let before = projection.project(&view).unwrap().unwrap();
    drop(projection);
    let serialized: serde_json::Value = yolo_paths::read_json(&path.join(STATE_FILE)).unwrap();
    assert!(serialized.get(RETIRED_CONTROLS_FIELD).is_none());
    assert!(
        serialized[ENTRIES_FIELD]
            .as_object()
            .unwrap()
            .values()
            .all(|entry| entry.get(CONTROL_WITNESS_FIELD).is_none())
    );
    let mut reopened = open(&path, hub);
    let after = reopened
        .project_with_observation(&view, Some(&observation(&view, IDENTITY_STEP)))
        .unwrap()
        .unwrap();
    assert_eq!(after.cursor().target(), before.cursor().target());
    assert_eq!(
        after.fields().capabilities.interrupt,
        Capability::Available {}
    );
}
