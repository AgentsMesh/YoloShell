use super::*;

#[test]
fn session_start_topology_requires_root_scope_and_the_right_session_boundary() {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    let initial = reducer.state().clone();
    started(&mut reducer, &identity);
    for source in [
        SessionStartSource::Startup,
        SessionStartSource::Resume,
        SessionStartSource::Clear,
        SessionStartSource::Fork,
        SessionStartSource::Compact,
    ] {
        for child in [false, true] {
            for foreign in [false, true] {
                let mut request = envelope(
                    &identity,
                    CODEX_VERSION,
                    crate::FIRST_SEQUENCE,
                    EventKind::SessionStart {
                        source: source.clone(),
                    },
                );
                request.agent_id = child.then(|| CHILD.to_owned());
                if foreign {
                    request.provider_session_id = FOREIGN_SESSION.into();
                }
                let initial_ok = !child && source != SessionStartSource::Compact;
                assert_eq!(
                    validate_topology(&initial, &request).is_ok(),
                    initial_ok,
                    "initial {source:?} {child} {foreign}"
                );
                let active_ok = !child
                    && source != SessionStartSource::Startup
                    && (source != SessionStartSource::Compact || !foreign);
                assert_eq!(
                    validate_topology(reducer.state(), &request).is_ok(),
                    active_ok,
                    "active {source:?} {child} {foreign}"
                );
            }
        }
    }
}

#[test]
fn child_topology_and_unknown_gap_recovery_never_manufacture_child_authority() {
    let identity = identity();
    let mut reducer = Reducer::new(identity.clone(), HookProviderKind::Codex, CODEX_VERSION).unwrap();
    started(&mut reducer, &identity);
    let mut state = reducer.state().clone();
    for event in [
        EventKind::SubagentStart,
        EventKind::SubagentStop,
        EventKind::Stop,
        EventKind::SessionEnd {
            reason: SessionEndReason::Other,
        },
    ] {
        let mut request = envelope(
            &identity,
            CODEX_VERSION,
            crate::FIRST_SEQUENCE,
            event.clone(),
        );
        assert!(!is_unknown_child_gap(&state, &request));
        request.agent_id = Some(CHILD.into());
        let start = matches!(event, EventKind::SubagentStart);
        let end = matches!(event, EventKind::SessionEnd { .. });
        assert_eq!(validate_topology(&state, &request).is_ok(), start);
        assert_eq!(is_unknown_child_gap(&state, &request), !start && !end);
        state.children.insert(
            CHILD.into(),
            ChildLifecycle {
                active: true,
                turn: TurnState::Running,
                attention: crate::AttentionState::None,
                active_tools: Default::default(),
                stop_candidate: None,
            },
        );
        assert_eq!(validate_topology(&state, &request).is_ok(), !start && !end);
        assert!(!is_unknown_child_gap(&state, &request));
        state.children.clear();
    }
    let mut foreign = envelope(
        &identity,
        CODEX_VERSION,
        crate::FIRST_SEQUENCE,
        EventKind::Stop,
    );
    foreign.provider_session_id = FOREIGN_SESSION.into();
    assert_eq!(
        validate_topology(&state, &foreign),
        Err(HookViolation::SessionMismatch)
    );
    assert_eq!(state.provider_session_id.as_deref(), Some(SESSION));
}
