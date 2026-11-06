#[test]
fn a_pointer_event_in_the_vacant_pane_does_not_revoke_the_creation_intent() {
    let mut fixture = super::creation_support::Fixture::new();
    fixture.state.mode = Mode::Terminal;
    fixture.state.bind_focused(None).unwrap();
    assert!(crate::shell_actions::create(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        "/tmp".into(),
        None,
    ));
    let Job::Call {
        generation,
        origin,
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("a shell creation must be dispatched");
    };
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"x".to_vec()),
    ));
    let epoch = fixture.state.ui_epoch;
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        }),
    );
    assert_eq!(fixture.state.ui_epoch, epoch);

    let creation = fixture.created();
    fixture.finish(
        ControlRequest {
            validity: None,
            generation,
            destination: origin,
            command: Command::Create { request },
        },
        creation,
    );
    assert!(fixture.state.initial_control.is_some());
    assert!(fixture.state.inputs.pregrant.iter().any(|item| {
        item.target == crate::terminal_input::PregrantTarget::Address(creation.address)
    }));

    let session = summary_with(creation.address, RuntimeStatus::Running);
    fixture
        .state
        .terminals
        .insert(creation.address, session.clone());
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    let grant = super::fixtures::grant(&session);
    fixture.state.record_grant(grant, Instant::now());
    crate::terminal_input::flush_pregrant(
        &mut fixture.state,
        &fixture.jobs,
        creation.address,
        grant,
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert_eq!(take_inputs(&mut fixture.incoming), [b"x".to_vec()]);
}

#[test]
fn blocked_pregrant_flush_retains_bytes_until_readiness_returns() {
    for blocked_by_resync in [true, false] {
        let (mut fixture, _request) = reattaching();
        typed(&mut fixture);
        assert_eq!(fixture.state.inputs.pregrant.len(), 1);
        if blocked_by_resync {
            fixture.state.mark_resync(fixture.target.address);
        } else {
            fixture.state.mark_offline(fixture.target.address);
        }
        fixture.state.record_grant(fixture.grant, Instant::now());
        crate::terminal_input::flush_pregrant(
            &mut fixture.state,
            &fixture.jobs,
            fixture.target.address,
            fixture.grant,
        );
        assert_eq!(fixture.state.inputs.pregrant.len(), 1);
        assert!(
            !fixture
                .state
                .inputs
                .snapshot_pending
                .contains(&fixture.target.address)
        );
        assert!(fixture.incoming.try_recv().is_err());

        fixture.state.clear_resync(fixture.target.address);
        fixture.state.clear_offline(fixture.target.address);
        crate::terminal_input::drain_pregrant(&mut fixture.state, &fixture.jobs);
        assert!(fixture.state.inputs.pregrant.is_empty());
        assert_eq!(only_input(&mut fixture), TYPED);
    }
}
