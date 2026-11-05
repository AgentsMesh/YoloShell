#[test]
fn legacy_viewport_without_control_status_shows_unknown_control_state() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let inventory = fixture.inventory();
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();

    terminal
        .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
        .unwrap();

    let content = contents(&terminal);
    assert!(content.contains(ControlBadge::Unavailable.glyph()));
    assert!(!content.contains(CONTROL_UNKNOWN_LABEL));
    assert!(!content.contains(OBSERVE_ONLY_CONTROL_LABEL));
}

#[test]
fn unsupported_empty_stale_and_provider_specific_states_are_distinct() {
    let fixture = Fixture::new();
    let terminals = fixture.terminals();
    let mut inventory = Inventory::default();
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    for (view, text) in [
        (View::Agents, "Agent inventory unavailable"),
        (View::Machines, "Machine inventory unavailable"),
    ] {
        inventory.view = view;
        terminal
            .draw(|frame| render(frame, &inventory, &terminals, None, "", frame.area()))
            .unwrap();
        assert!(contents(&terminal).contains(text));
    }
    inventory.replace(Some(Vec::new()), Some(Vec::new()));
    for (view, text) in [
        (View::Terminals, "No terminals"),
        (View::Agents, "No agent runs"),
        (View::Machines, "No machines"),
    ] {
        inventory.view = view;
        terminal
            .draw(|frame| {
                render(
                    frame,
                    &inventory,
                    &terminals,
                    None,
                    NONMATCHING_FILTER,
                    frame.area(),
                )
            })
            .unwrap();
        assert!(contents(&terminal).contains(text));
    }
    inventory = fixture.inventory();
    for (source, recovery, expected_source, expected_recovery) in [
        (
            ObservationSource::ProcessObserver,
            RecoveryStatus::Unknown,
            "Process",
            "Resume unknown",
        ),
        (
            ObservationSource::Connector,
            RecoveryStatus::Unsupported,
            "Connector",
            "Resume unsupported",
        ),
        (
            ObservationSource::ProviderHook,
            RecoveryStatus::Unavailable,
            "Hooks",
            "Resume unavailable",
        ),
    ] {
        let mut fields = fixture.agent.fields().clone();
        fields.source = source;
        fields.recovery = recovery;
        fields.freshness = Freshness::Stale;
        fields.reachability = Reachability::Unreachable;
        fields.lifecycle = Lifecycle::Exited { exit_code: None };
        fields.activity = Activity::Idle;
        inventory.history = true;
        inventory.replace(Some(vec![AgentSnapshot::new(fields).unwrap()]), None);
        inventory.details = Some(fixture.agent.cursor().target());
        terminal
            .draw(|frame| {
                assert!(render_details(
                    frame,
                    &inventory,
                    &terminals,
                    None,
                    frame.area()
                ));
            })
            .unwrap();
        let content = contents(&terminal);
        for text in [
            expected_source,
            expected_recovery,
            "Stale",
            "Unreachable",
            "Exited",
            &fixture.hub.hub_id.to_string(),
            &fixture.agent.cursor().target().address.agent_id.to_string(),
        ] {
            assert!(content.contains(text));
        }
    }
}
