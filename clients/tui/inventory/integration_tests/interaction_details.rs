use super::constants::*;
use super::fixtures::Fixture;
use ratatui::{Terminal, backend::TestBackend};
use uuid::Uuid;
use yolo_hub_agent_protocol::{
    Activity, AgentSnapshot, Capability, Interaction, InteractionChoice, InteractionKind,
    InteractionReference,
};
use yolo_tui_inventory::{INTERACTION_ANSWER_HINT, render_details};

#[test]
fn details_show_the_prompt_and_numbered_choices() {
    let fixture = Fixture::new();
    let target = fixture.agent.cursor().target();
    let mut fields = fixture.agent.fields().clone();
    fields.activity = Activity::WaitingForApproval;
    fields.capabilities.respond_interaction = Capability::Available {};
    fields.interaction = Some(
        Interaction::with_prompt(
            InteractionReference {
                target,
                id: Uuid::new_v4(),
            },
            InteractionKind::Approval,
            vec![
                InteractionChoice {
                    id: "allow".into(),
                    label: "Allow".into(),
                },
                InteractionChoice {
                    id: "deny".into(),
                    label: "Deny".into(),
                },
            ],
            "Bash: cargo test".into(),
        )
        .unwrap(),
    );
    let agent = AgentSnapshot::new(fields).unwrap();
    let mut inventory = fixture.inventory();
    inventory.replace(Some(vec![agent]), Some(vec![fixture.hub.clone()]));
    inventory.details = Some(target);
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| {
            assert!(render_details(
                frame,
                &inventory,
                &fixture.terminals(),
                None,
                frame.area()
            ));
        })
        .unwrap();
    let content: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(content.contains("Bash: cargo test"), "{content}");
    assert!(content.contains("1. Allow"), "{content}");
    assert!(content.contains("2. Deny"), "{content}");
    assert!(content.contains(INTERACTION_ANSWER_HINT), "{content}");
}
