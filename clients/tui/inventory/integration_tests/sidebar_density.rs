use crate::{constants::*, fixtures::Fixture, render_support::render};
use ratatui::{Terminal, backend::TestBackend};
use yolo_hub_agent_protocol::{AgentSnapshot, Freshness, Reachability};
use yolo_tui_inventory::{HEALTH_WARNING_GLYPH, Inventory, SPINNER_FRAMES, render_details};

const VISIBLE_AGENT_COUNT: usize = 5;
const COMPACT_HEIGHT: u16 = 17;

#[test]
fn five_agents_fit_without_scrolling_in_seventeen_rows() {
    let fixtures = (0..VISIBLE_AGENT_COUNT)
        .map(|_| Fixture::new())
        .collect::<Vec<_>>();
    let terminals = fixtures
        .iter()
        .map(|fixture| (fixture.session.fields().address, fixture.session.clone()))
        .collect();
    let mut inventory = Inventory::default();
    inventory.current = true;
    inventory.replace(
        Some(
            fixtures
                .iter()
                .map(|fixture| fixture.agent.clone())
                .collect(),
        ),
        Some(fixtures.iter().map(|fixture| fixture.hub.clone()).collect()),
    );
    inventory.reconcile("", &terminals);
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, COMPACT_HEIGHT)).unwrap();
    terminal
        .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
        .unwrap();
    let content = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>();
    assert_eq!(content.matches(PROVIDER_KIND).count(), VISIBLE_AGENT_COUNT);
    assert_eq!(
        content.matches(SPINNER_FRAMES[0]).count(),
        VISIBLE_AGENT_COUNT
    );
    assert!(!content.contains(AGENT_WORKING_DIRECTORY));
    assert!(content.contains("Machine "));
    assert!(!content.contains("Terminals"));
}

#[test]
fn abnormal_observations_remain_explicit_without_repeating_healthy_metadata() {
    let fixture = Fixture::new();
    let mut inventory = fixture.inventory();
    for (current, freshness, reachability, warning) in [
        (false, Freshness::Current, Reachability::Reachable, "Stale"),
        (true, Freshness::Stale, Reachability::Reachable, "Stale"),
        (
            true,
            Freshness::Current,
            Reachability::Unreachable,
            "Unreachable",
        ),
    ] {
        let mut fields = fixture.agent.fields().clone();
        fields.freshness = freshness;
        fields.reachability = reachability;
        inventory.current = current;
        inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
        inventory.history = true;
        let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
        terminal
            .draw(|frame| {
                render(
                    frame,
                    &inventory,
                    &fixture.terminals(),
                    None,
                    "",
                    frame.area(),
                )
            })
            .unwrap();
        let content = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(content.contains(HEALTH_WARNING_GLYPH));
        assert!(!content.contains("Working"));
        assert!(!content.contains(LAST_KNOWN_DIRECTORY_LABEL));
        assert!(!content.contains("Hooks"));
        assert!(!content.contains("Resume available"));

        inventory.details = Some(fixture.agent.cursor().target());
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
        let details = terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(details.contains(&format!("Working | {warning}")) || details.contains("Working"));
        assert!(details.contains(warning));
        assert!(details.contains(LAST_KNOWN_DIRECTORY_LABEL));
    }
}
