use super::fixtures;
use crate::{Mode, respond_actions, State};
use std::time::Instant;
use tokio::sync::mpsc;
use yolo_hub_agent_protocol::{
    AgentSnapshot, Capability, Interaction, InteractionChoice, InteractionKind,
    InteractionReference,
};

const QUEUE: usize = 8;

fn pending() -> State {
    let (agent, session) = fixtures::agent();
    let target = agent.cursor().target();
    let mut fields = agent.fields().clone();
    fields.capabilities.respond_interaction = Capability::Available {};
    fields.activity = yolo_hub_agent_protocol::Activity::WaitingForApproval;
    fields.interaction = Some(
        Interaction::with_prompt(
            InteractionReference { target, id: uuid::Uuid::new_v4() },
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
    let mut state = crate::client_capability_support::state();
    state.mode = Mode::Inventory;
    state.control_generation = Some(uuid::Uuid::new_v4());
    state.inventory.replace(Some(vec![agent]), Some(vec![]));
    state.inventory.current = true;
    state.inventory.view = yolo_tui_inventory::View::Agents;
    state.inventory.selected_agent = state.inventory.agents.keys().next().copied();
    state.inventory.details = state.inventory.selected_agent;
    state.replace_terminals(vec![session.clone()]);
    state.bind_focused(Some(session.fields().address)).unwrap();
    let grant = fixtures::grant(&session);
    state.principal = grant.fields().principal;
    state.record_grant(grant, Instant::now());
    state
}

#[test]
fn a_second_digit_press_cannot_dispatch_a_duplicate_respond() {
    let mut state = pending();
    let (jobs, mut incoming) = mpsc::channel(QUEUE);
    assert!(respond_actions::choose(&mut state, &jobs, 0));
    assert!(matches!(
        incoming.try_recv().unwrap(),
        crate::network::Job::Call {
            command: yolo_hub_rpc_protocol::Command::AgentControl { .. },
            ..
        }
    ));
    assert!(!respond_actions::choose(&mut state, &jobs, 0));
    assert!(incoming.try_recv().is_err());
    assert_eq!(state.respond_pending.len(), 1);
}
