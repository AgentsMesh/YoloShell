use super::{
    constants::*,
    control_support::*,
    fixtures::{directory, open},
    view::view,
};
use yolo_agent_control_contract::{NativeControlObservation, NativeControlWitness};
use yolo_hub_agent_protocol::Capability;

#[test]
fn private_witness_enables_only_interrupt_without_changing_public_agent_identity() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let view = view();
    let before = projection.project(&view).unwrap().unwrap();
    assert_eq!(
        before.fields().capabilities.interrupt,
        Capability::Unsupported {}
    );
    let observed = observation(&view, IDENTITY_STEP);
    let current = projection
        .project_with_observation(&view, Some(&observed))
        .unwrap()
        .unwrap();
    assert_eq!(current.cursor().target(), before.cursor().target());
    assert_eq!(current.cursor().stream(), before.cursor().stream());
    assert!(current.cursor().sequence() > before.cursor().sequence());
    assert_eq!(
        current.fields().capabilities.interrupt,
        Capability::Available {}
    );
    assert_eq!(
        current.fields().capabilities.respond_interaction,
        Capability::Unsupported {}
    );
    assert_eq!(
        projection.control_witness(&request(&current)).unwrap(),
        Some(observed.witness().clone())
    );
    assert!(projection.control_witness(&request(&before)).is_err());
    assert!(
        serde_json::to_value(&current)
            .unwrap()
            .get(CONTROL_WITNESS_FIELD)
            .is_none()
    );
}

#[test]
fn pending_interaction_is_projected_with_respond_capability() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let view = view();
    let interaction = uuid::Uuid::new_v4();
    let observed = observation_with_interaction(&view, IDENTITY_STEP, interaction);
    let current = projection
        .project_with_observation(&view, Some(&observed))
        .unwrap()
        .unwrap();
    let fields = current.fields();
    let pending = fields.interaction.as_ref().unwrap();
    assert_eq!(pending.reference().id, interaction);
    assert_eq!(pending.prompt(), "Bash: cargo test");
    assert_eq!(pending.kind(), yolo_hub_agent_protocol::InteractionKind::Approval);
    assert_eq!(fields.activity, yolo_hub_agent_protocol::Activity::WaitingForApproval);
    assert_eq!(
        fields.capabilities.respond_interaction,
        Capability::Available {}
    );
    assert_eq!(
        projection
            .control_witness(&respond_request(&current))
            .unwrap(),
        Some(observed.witness().clone())
    );

    let idle = observation(&view, IDENTITY_STEP + IDENTITY_STEP);
    let cleared = projection
        .project_with_observation(&view, Some(&idle))
        .unwrap()
        .unwrap();
    assert!(cleared.fields().interaction.is_none());
    assert_eq!(
        cleared.fields().capabilities.respond_interaction,
        Capability::Unsupported {}
    );
    assert!(projection.control_witness(&respond_request(&current)).is_err());
}

#[test]
fn changed_native_authority_advances_cursor_even_when_visible_binding_state_is_identical() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let view = view();
    let old = observation(&view, IDENTITY_STEP);
    let first = projection
        .project_with_observation(&view, Some(&old))
        .unwrap()
        .unwrap();
    let replacement = observation(&view, IDENTITY_STEP + IDENTITY_STEP);
    let current = projection
        .project_with_observation(&view, Some(&replacement))
        .unwrap()
        .unwrap();
    assert_eq!(current.cursor().target(), first.cursor().target());
    assert_eq!(current.cursor().stream(), first.cursor().stream());
    assert_eq!(
        current.cursor().sequence(),
        first.cursor().sequence() + IDENTITY_STEP
    );
    let mut expected = first.fields().clone();
    expected.cursor = current.cursor();
    assert_eq!(current.fields(), &expected);
    assert!(projection.control_witness(&request(&first)).is_err());
    assert_eq!(
        projection.control_witness(&request(&current)).unwrap(),
        Some(replacement.witness().clone())
    );
    assert_eq!(
        projection
            .project_with_observation(&view, Some(&replacement))
            .unwrap(),
        Some(current)
    );
}

#[test]
fn foreign_worker_runtime_or_binding_observation_cannot_change_projection() {
    let (_root, path, hub) = directory();
    let mut projection = open(&path, hub);
    let view = view();
    let original = observation(&view, IDENTITY_STEP);
    let before = projection
        .project_with_observation(&view, Some(&original))
        .unwrap()
        .unwrap();
    for change_worker in [false, true] {
        let mut fields = original.witness().fields().clone();
        if change_worker {
            fields.worker_id = uuid::Uuid::new_v4();
        } else {
            fields.runtime_epoch = uuid::Uuid::new_v4();
        }
        let foreign = NativeControlObservation::new(
            original.binding().clone(),
            NativeControlWitness::new(fields).unwrap(),
            false,
        )
        .unwrap();
        assert!(
            projection
                .project_with_observation(&view, Some(&foreign))
                .is_err()
        );
        assert_eq!(
            projection.get(before.cursor().target().address).unwrap(),
            Some(before.clone())
        );
    }
    let mut other = view.clone();
    other.agent.as_mut().unwrap().cwd = CHANGED_AGENT_DIRECTORY.into();
    assert!(
        projection
            .project_with_observation(&view, Some(&observation(&other, IDENTITY_STEP)))
            .is_err()
    );
    assert_eq!(
        projection.get(before.cursor().target().address).unwrap(),
        Some(before)
    );
}
