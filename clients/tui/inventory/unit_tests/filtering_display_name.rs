use crate::constants::{PROVIDER_DISPLAY_NAME, PROVIDER_KIND};
use crate::fixtures::Fixture;
use yolo_hub_agent_protocol::{AgentSnapshot, ObservedProvider};

#[test]
fn provider_display_name_is_searchable() {
    let fixture = Fixture::new();
    let mut fields = fixture.agent.fields().clone();
    fields.provider = ObservedProvider::new(PROVIDER_KIND.into(), None, None)
        .unwrap()
        .with_display_name(Some(PROVIDER_DISPLAY_NAME.into()))
        .unwrap();
    let mut inventory = fixture.inventory();
    inventory.replace(
        Some(vec![AgentSnapshot::new(fields).unwrap()]),
        Some(vec![fixture.hub.clone()]),
    );
    assert_eq!(
        inventory
            .visible_agents(PROVIDER_DISPLAY_NAME, &fixture.terminals())
            .len(),
        1
    );
}
