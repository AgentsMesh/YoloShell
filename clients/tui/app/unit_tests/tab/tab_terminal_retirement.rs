#[test]
fn exit_event_hides_a_background_terminal_and_agent_without_a_bound_tab() {
    let (agent, session) = fixtures::agent();
    let target = session.fields().runtime.unwrap().target;
    let mut state = state_with(&[session]);
    state.inventory.replace(Some(vec![agent]), Some(vec![]));
    state.reconcile_selection();
    assert_eq!(
        state.inventory.visible_agents("", &state.terminals).len(),
        1
    );
    assert_eq!(
        state
            .inventory
            .visible_terminals("", &state.terminals)
            .len(),
        1
    );

    crate::observation_events::terminal(
        &mut state,
        TerminalEvent::Exited {
            cursor: OutputCursor {
                target,
                after_sequence: u64::default(),
            },
            code: Some(i32::default()),
            signal: None,
        },
    );

    assert!(
        state
            .inventory
            .visible_agents("", &state.terminals)
            .is_empty()
    );
    assert!(
        state
            .inventory
            .visible_terminals("", &state.terminals)
            .is_empty()
    );
    assert_eq!(state.inventory.selected_agent, None);
    state.inventory.history = true;
    assert_eq!(
        state
            .inventory
            .visible_terminals("", &state.terminals)
            .len(),
        1
    );
}

#[test]
fn opening_a_gone_terminal_keeps_the_history_picker_instead_of_binding_a_pane() {
    let session = fixtures::session();
    let address = session.fields().address;
    let mut state = state_with(&[ended(&session)]);
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);

    tabs::open(&mut state, &jobs, address);

    assert!(state.terminal_picker.is_some());
    assert!(state.inventory.history);
    assert_eq!(state.selected, Some(address));
    assert_eq!(state.focused_terminal(), None);
    assert!(state.status.contains("R to resume"), "{}", state.status);
    assert!(incoming.try_recv().is_err());
}

#[test]
fn retiring_a_dead_terminal_releases_its_presentation_surface() {
    let session = fixtures::session();
    let address = session.fields().address;
    let target = session.fields().runtime.unwrap().target;
    let mut state = state_with(std::slice::from_ref(&session));
    let size = yolo_hub_terminal_protocol::TerminalSize::new(80, 24).unwrap();
    state.surfaces.insert(
        address,
        yolo_tui_terminal::TerminalSurface::new(
            &yolo_hub_terminal_protocol::TerminalSnapshot::new(
                yolo_hub_terminal_protocol::TerminalSnapshotFields {
                    target,
                    barrier_sequence: 0,
                    size,
                    data: b"retained output".to_vec(),
                },
            )
            .unwrap(),
        ),
    );
    state.set_last_size(address, size);
    state.mark_resize_pending(address);

    state.replace_terminals(vec![ended(&session)]);

    assert!(!state.surfaces.contains_key(&address));
    assert!(!state.last_size(address).is_some());
    assert!(!state.is_resize_pending(address));
    assert!(state.terminals.contains_key(&address));
}

#[test]
fn archived_record_retires_the_binding_and_stays_in_history() {
    let session = fixtures::session();
    let address = session.fields().address;
    let mut state = state_with(std::slice::from_ref(&session));
    bind_as_terminal_tab(&mut state, address);
    state.mode = Mode::Terminal;
    let mut fields = session.fields().clone();
    fields.archived = true;
    let archived = TerminalSummary::new(fields).unwrap();

    state.replace_terminals(vec![archived]);

    assert_eq!(state.focused_terminal(), None);
    assert!(
        state
            .tabs
            .active()
            .view
            .layout
            .panes()
            .iter()
            .all(|pane| pane.terminal.is_none())
    );
    assert!(
        state
            .inventory
            .visible_terminals("", &state.terminals)
            .is_empty()
    );
    state.inventory.history = true;
    assert_eq!(
        state.inventory.visible_terminals("", &state.terminals),
        vec![address]
    );
}
