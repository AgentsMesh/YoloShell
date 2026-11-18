use super::fixtures::QWEN_KIND;
use super::interrupt_support;
use crate::terminal_status;
use yolo_hub_agent_protocol::{Activity, AgentSnapshot, ObservedProvider};

#[test]
fn activity_status_uses_the_provider_display_name() {
    let mut state = interrupt_support::ready();
    let id = state.inventory.agents.keys().next().copied().unwrap();
    let mut fields = state.inventory.agents[&id].fields().clone();
    fields.provider = ObservedProvider::new(QWEN_KIND.into(), None, None)
        .unwrap()
        .with_display_name(Some("Qwen Code".into()))
        .unwrap();
    fields.activity = Activity::Working;
    state
        .inventory
        .agents
        .insert(id, AgentSnapshot::new(fields).unwrap());

    let line = terminal_status::line(&state, None);
    let text: String = line
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect();

    assert!(text.contains("Qwen Code working"), "{text}");
    assert!(!text.contains("qwen_code working"), "{text}");
}
