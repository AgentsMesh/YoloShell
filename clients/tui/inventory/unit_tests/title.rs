use crate::constants::QWEN_CODE_KIND;
use crate::fixtures::Fixture;
use yolo_hub_agent_protocol::{AgentSnapshot, ObservedProvider};
use yolo_hub_terminal_protocol::TerminalSummary;
use yolo_tui_inventory::agent_title;

#[test]
fn provider_display_name_is_the_fallback_agent_title() {
    let fixture = Fixture::new();
    let mut fields = fixture.agent.fields().clone();
    fields.provider = ObservedProvider::new(QWEN_CODE_KIND.into(), None, None)
        .unwrap()
        .with_display_name(Some("Qwen Code".into()))
        .unwrap();
    let agent = AgentSnapshot::new(fields).unwrap();
    let address = agent.fields().terminal.address;
    let mut terminals = fixture.terminals();
    let mut terminal_fields = terminals[&address].fields().clone();
    terminal_fields.presentation.title = None;
    terminals.insert(address, TerminalSummary::new(terminal_fields).unwrap());

    assert_eq!(agent_title(&agent, &terminals), "Qwen Code");
}
