use crate::{constants::*, fixtures::Fixture, render_support::render};
use ratatui::{Terminal, backend::TestBackend};
use std::collections::BTreeMap;
use yolo_hub_agent_protocol::{
    Activity, AgentSnapshot, Freshness, Lifecycle, ObservationSource, Reachability, RecoveryStatus,
};
use yolo_tui_inventory::{
    CONTROL_UNKNOWN_LABEL, ControlBadge, HUB_REACHABLE_GLYPH, HUB_UNKNOWN_GLYPH,
    HUB_UNREACHABLE_GLYPH, Inventory, OBSERVE_ONLY_CONTROL_LABEL, SPINNER_FRAMES, View,
    render_details, render_viewport,
};

fn contents(terminal: &Terminal<TestBackend>) -> String {
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

#[test]
fn agent_rows_keep_title_and_compact_state_without_metadata_noise() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    for (activity, marker) in [
        (Activity::Unknown, None),
        (Activity::Idle, None),
        (Activity::Working, Some(SPINNER_FRAMES[0])),
        (Activity::WaitingForInput, Some("‖")),
        (Activity::WaitingForApproval, Some("!")),
    ] {
        let mut fields = fixture.agent.fields().clone();
        fields.activity = activity;
        inventory.replace(
            Some(vec![AgentSnapshot::new(fields).unwrap()]),
            Some(vec![fixture.hub.clone()]),
        );
        terminal
            .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
            .unwrap();
        let text_content = contents(&terminal);
        assert!(text_content.contains(PROVIDER_KIND), "{text_content}");
        if let Some(marker) = marker {
            assert!(text_content.contains(marker), "{text_content}");
        }
        for hidden in [
            "Hooks",
            "Resume available",
            "Reachable",
            "Working",
            "Idle",
            "Unknown",
            "Waiting for input",
            "Waiting for approval",
            "Running",
            "Terminals",
            AGENT_WORKING_DIRECTORY,
            &fixture.hub.hub_id.to_string(),
            &fixture.agent.cursor().target().address.agent_id.to_string(),
        ] {
            assert!(
                !text_content.contains(hidden),
                "unexpected {hidden}: {text_content}"
            );
        }
    }
}

#[test]
fn hub_rows_use_local_hostname_and_a_single_status_glyph() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = fixture.inventory();
    inventory.set_local_hub(fixture.hub.hub_id, Some("build-host".into()));
    inventory.view = View::Hubs;
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal
        .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
        .unwrap();
    let content = contents(&terminal);
    assert!(content.contains("Hub build-host"), "{content}");
    assert!(content.contains(HUB_REACHABLE_GLYPH), "{content}");
    assert!(!content.contains("Reachable"), "{content}");
    assert!(
        !content.contains(&fixture.hub.hub_id.to_string()),
        "{content}"
    );

    inventory
        .hubs
        .get_mut(&fixture.hub.hub_id)
        .unwrap()
        .reachable = false;
    terminal
        .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
        .unwrap();
    assert!(contents(&terminal).contains(HUB_UNREACHABLE_GLYPH));

    inventory.current = false;
    terminal
        .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
        .unwrap();
    assert!(contents(&terminal).contains(HUB_UNKNOWN_GLYPH));
}

#[test]
fn agent_control_and_spinner_occupy_fixed_icon_slots() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let address = fixture.agent.fields().terminal.address;
    let mut inventory = fixture.inventory();
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    let writable = BTreeMap::from([(address, ControlBadge::Writable)]);
    terminal
        .draw(|frame| {
            render_viewport(
                frame,
                &inventory,
                &terminals,
                None,
                "",
                &writable,
                0,
                frame.area(),
            );
        })
        .unwrap();
    let content = contents(&terminal);
    assert!(
        content.contains(ControlBadge::Writable.glyph()),
        "{content}"
    );
    assert!(content.contains(SPINNER_FRAMES[0]), "{content}");
    assert!(!content.contains(AGENT_WORKING_DIRECTORY), "{content}");

    terminal
        .draw(|frame| {
            render_viewport(
                frame,
                &inventory,
                &terminals,
                None,
                "",
                &writable,
                1,
                frame.area(),
            );
        })
        .unwrap();
    assert!(contents(&terminal).contains(SPINNER_FRAMES[1]));

    let mut fields = fixture.agent.fields().clone();
    fields.activity = Activity::Idle;
    inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
    let read_only = BTreeMap::from([(address, ControlBadge::ReadOnly)]);
    terminal
        .draw(|frame| {
            render_viewport(
                frame,
                &inventory,
                &terminals,
                None,
                "",
                &read_only,
                0,
                frame.area(),
            );
        })
        .unwrap();
    let content = contents(&terminal);
    assert!(
        content.contains(ControlBadge::ReadOnly.glyph()),
        "{content}"
    );
    assert!(!content.contains(SPINNER_FRAMES[0]), "{content}");
}


include!("rendering_control_states.rs");
