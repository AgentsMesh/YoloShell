#[test]
fn typing_into_a_new_tab_before_its_terminal_binds_keeps_the_bytes() {
    let mut fixture = super::creation_support::Fixture::new();
    fixture.state.mode = Mode::Terminal;
    assert!(crate::tabs::creation::create(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        "/tmp".into(),
        None,
        crate::tabs::creation::Placement::Tab,
    ));
    let Job::Call {
        generation,
        origin,
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("a new tab must dispatch one creation");
    };
    let epoch = fixture.state.ui_epoch;
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    );
    assert_eq!(fixture.state.ui_epoch, epoch, "typing is not navigation");
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    assert!(matches!(
        fixture.state.inputs.pregrant[0].target,
        crate::terminal_input::PregrantTarget::Creation(_)
    ));

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
    assert!(matches!(
        fixture.state.inputs.pregrant[0].target,
        crate::terminal_input::PregrantTarget::Address(address)
            if address == creation.address
    ));
    let session = summary_with(creation.address, RuntimeStatus::Running);
    fixture
        .state
        .terminals
        .insert(creation.address, session.clone());
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    let mut acquire = None;
    while let Ok(job) = fixture.incoming.try_recv() {
        if let Job::InitialAcquire {
            generation,
            origin,
            request,
            validity,
        } = job
        {
            acquire = Some((generation, origin, request, validity));
            break;
        }
    }
    let Some((generation, origin, request, validity)) = acquire else {
        panic!("the bound new tab must acquire its terminal");
    };
    let grant = super::fixtures::grant(&session);
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request: ControlRequest {
                generation: Some(generation),
                destination: origin,
                command: Command::Lease { request },
                validity: Some(validity),
            },
            result: Ok(Reply::Lease { grant }),
            connected: true,
        }),
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert_eq!(take_inputs(&mut fixture.incoming), [b"x".to_vec()]);
}

#[test]
fn bound_creation_adopts_input_after_presentation_authority_is_lost() {
    let mut fixture = super::creation_support::Fixture::new();
    fixture.state.mode = Mode::Terminal;
    assert!(crate::tabs::creation::create(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        "/tmp".into(),
        None,
        crate::tabs::creation::Placement::Tab,
    ));
    let Job::Call {
        generation,
        origin,
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("a new tab must dispatch one creation");
    };
    fixture.state.ui_epoch = fixture.state.ui_epoch.saturating_add(1);
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        Event::Key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE)),
    );
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

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
    assert!(fixture.state.initial_control.is_none());
    assert!(matches!(
        fixture.state.inputs.pregrant[0].target,
        crate::terminal_input::PregrantTarget::Address(address)
            if address == creation.address
    ));
    // Before the catalogue carries the new terminal, the focused bound pane is
    // still Unavailable; a second keystroke must be retained rather than lost.
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"y".to_vec()),
    ));
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    let session = summary_with(creation.address, RuntimeStatus::Running);
    fixture
        .state
        .terminals
        .insert(creation.address, session.clone());
    let grant = super::fixtures::grant(&session);
    fixture.state.record_grant(grant, Instant::now());
    crate::terminal_input::flush_pregrant(
        &mut fixture.state,
        &fixture.jobs,
        creation.address,
        grant,
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert_eq!(take_inputs(&mut fixture.incoming), [b"xy".to_vec()]);
}
