#[test]
fn rapid_codex_wheel_burst_opens_once_and_never_toggles_the_transcript_closed() {
    let (mut state, _, second) = state();
    bind_codex(&mut state, &second);
    writable(&mut state, &second, &codex_main());
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(
        bytes(incoming.try_recv().unwrap()),
        CODEX_OPEN_UP,
        "the first notch opens the transcript"
    );
    for _ in 0..4 {
        pointer::handle(
            &mut state,
            &jobs,
            mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
        );
        assert_eq!(
            bytes(incoming.try_recv().unwrap()),
            CODEX_UP,
            "a burst before the overlay redraw must not send another Ctrl-T"
        );
    }
}

#[test]
fn codex_draft_without_agent_observation_still_opens_its_transcript() {
    let (mut state, _, second) = state();
    writable(&mut state, &second, &codex_draft());
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), CODEX_OPEN_UP);
}

// Positive coverage for the bottom-block detector: a multi-line draft must not
// become a false negative. The stale-shell and scrolled-back tests below guard
// the broader old fragment-matching rule that this detector replaced.
#[test]
fn codex_multiline_draft_keeps_wheel_scrolling_available() {
    let (mut state, _, second) = state();
    writable(
        &mut state,
        &second,
        &codex_screen(
            "› first line\r\nsecond line\r\nthird line\r\n\r\n  gpt-6-astra max · ~/Works/Hello\r\n",
        ),
    );
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert_eq!(bytes(incoming.try_recv().unwrap()), CODEX_OPEN_UP);
}

#[test]
fn stale_codex_observation_does_not_inject_keys_into_the_shell() {
    for screen in [
        codex_screen("$ "),
        // Codex exited without clearing its last frame; the shell prompt is
        // now below that frame, so its footer no longer owns the bottom rows.
        codex_screen("› hello\r\n\r\n  gpt-6-astra max · ~/Works/Hello\r\n$ "),
    ] {
        let (mut state, _, second) = state();
        bind_codex(&mut state, &second);
        writable(&mut state, &second, &screen);
        let view = state.presentation.panes[1];
        let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
        pointer::handle(
            &mut state,
            &jobs,
            mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
        );
        assert!(incoming.try_recv().is_err());
        assert!(
            state.surfaces[&second.fields().address]
                .screen()
                .scrollback()
                > 0,
            "the shell keeps its own local scrollback"
        );
    }
}

#[test]
fn scrolled_back_codex_frame_never_generates_provider_keys() {
    let (mut state, _, second) = state();
    writable(&mut state, &second, &codex_main());
    let address = second.fields().address;
    state.surfaces.get_mut(&address).unwrap().set_scrollback(1);
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollUp, view.inner.x, view.inner.y),
    );
    assert!(incoming.try_recv().is_err());
}

#[test]
fn codex_wheel_down_at_the_live_tail_does_not_open_the_transcript() {
    let (mut state, _, second) = state();
    bind_codex(&mut state, &second);
    writable(&mut state, &second, &codex_main());
    let view = state.presentation.panes[1];
    let (jobs, mut incoming) = tokio::sync::mpsc::channel(crate::constants::QUEUE_CAPACITY);
    pointer::handle(
        &mut state,
        &jobs,
        mouse(MouseEventKind::ScrollDown, view.inner.x, view.inner.y),
    );
    assert!(incoming.try_recv().is_err());
    assert_eq!(
        state.surfaces[&second.fields().address]
            .screen()
            .scrollback(),
        0
    );
}

fn rewrite_surface(state: &mut crate::State, session: &TerminalSummary, data: &[u8]) {
    let address = session.fields().address;
    let view = state
        .presentation
        .panes
        .iter()
        .find(|pane| pane.terminal == Some(address))
        .unwrap();
    let size = TerminalSize::new(view.inner.width, view.inner.height).unwrap();
    state.set_last_size(address, size);
    let snapshot = TerminalSnapshot::new(TerminalSnapshotFields {
        target: session.fields().runtime.unwrap().target,
        barrier_sequence: 0,
        size,
        data: data.to_vec(),
    })
    .unwrap();
    state
        .surfaces
        .insert(address, TerminalSurface::new(&snapshot));
}

fn bind_codex(state: &mut crate::State, session: &TerminalSummary) {
    use yolo_hub_agent_protocol::ObservationCursor;
    use yolo_hub_identity::{AgentAddress, AgentTarget};
    let (agent, _) = fixtures::agent();
    let target = session.fields().runtime.unwrap().target;
    let agent_target = AgentTarget {
        address: AgentAddress {
            hub_id: target.address.hub_id,
            agent_id: agent.fields().cursor.target().address.agent_id,
        },
        execution_id: agent.fields().cursor.target().execution_id,
    };
    let mut fields = agent.fields().clone();
    fields.cursor = ObservationCursor::new(
        agent_target,
        fields.cursor.stream(),
        fields.cursor.sequence(),
    )
    .unwrap();
    fields.terminal = target;
    fields.lifecycle = Lifecycle::Running {};
    state.inventory.replace(
        Some(vec![AgentSnapshot::new(fields).unwrap()]),
        Some(Vec::new()),
    );
    state.inventory.current = true;
    let mut terminal = session.fields().clone();
    terminal.agent = Some(agent_target);
    state.terminals.insert(
        session.fields().address,
        TerminalSummary::new(terminal).unwrap(),
    );
}
