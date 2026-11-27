use super::{
    fixtures::{directory, open_with},
    view::view,
};
use yolo_agent_provider_contract::ProjectionPolicies;
use yolo_hub_agent_protocol::{Activity, ObservationSource};

#[test]
fn process_observer_policy_never_claims_native_hook_lifecycle() {
    let (_root, path, hub) = directory();
    let mut store = open_with(&path, hub, ProjectionPolicies::none());
    let snapshot = store.project(&view()).unwrap().unwrap();
    assert_eq!(snapshot.fields().source, ObservationSource::ProcessObserver);
    assert_eq!(snapshot.fields().activity, Activity::Unknown);
}

#[test]
fn provider_display_name_comes_from_the_projection_policy_catalog() {
    let (_root, path, hub) = directory();
    let mut policies = ProjectionPolicies::none();
    policies.insert_display_name(yolo_agent_contract::AgentKind::Codex, "Codex".into());
    let mut store = open_with(&path, hub, policies);
    let snapshot = store.project(&view()).unwrap().unwrap();
    assert_eq!(snapshot.fields().provider.display_name(), Some("Codex"));
}
