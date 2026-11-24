#[test]
fn bytes_typed_while_the_new_shell_reports_starting_wait_for_its_grant() {
    let (mut fixture, creation, _) = created_vacant_shell(RuntimeStatus::Starting, b"x");
    // The first prompt can render before the catalogue reports Running.
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"y".to_vec()),
    ));
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

    let running = summary_with(creation.address, RuntimeStatus::Running);
    fixture
        .state
        .terminals
        .insert(creation.address, running.clone());
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    let grant = super::fixtures::grant(&running);
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

#[test]
fn bytes_typed_before_the_created_terminal_is_catalogued_wait_for_its_grant() {
    let (mut fixture, creation, session) = created_vacant_shell(RuntimeStatus::Running, b"x");
    fixture.state.terminals.remove(&creation.address);
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"y".to_vec()),
    ));
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

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
    assert_eq!(take_inputs(&mut fixture.incoming), [b"xy".to_vec()]);
}

#[test]
fn startup_attach_keeps_bytes_typed_before_the_catalogue_is_ready() {
    let mut fixture =
        super::startup_support::Fixture::new(StartupAction::Attach { selector: None });
    let shell = super::startup_support::session(fixture.hub, RuntimeStatus::Running);
    fixture.state.replace_terminals(vec![shell.clone()]);
    for code in [KeyCode::Char('x'), KeyCode::Enter] {
        crate::actions::handle(
            &mut fixture.state,
            &fixture.jobs,
            fixture.hub,
            Event::Key(KeyEvent::new(code, KeyModifiers::NONE)),
        );
    }
    assert_eq!(fixture.state.mode, Mode::Inventory);
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

    fixture.ready();
    fixture.apply();
    assert_eq!(fixture.state.mode, Mode::Terminal);
    let Job::InitialAcquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("startup attach must acquire the bound terminal");
    };
    let grant = super::fixtures::grant(&shell);
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
    assert_eq!(take_inputs(&mut fixture.incoming), [b"x\r".to_vec()]);
}

#[test]
fn startup_new_shell_keeps_bytes_typed_before_the_creation_result() {
    let mut fixture = super::startup_support::Fixture::new(StartupAction::NewShell {
        cwd: "/tmp".into(),
        name: None,
    });
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.hub,
        Event::Key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE)),
    );
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

    fixture.ready();
    fixture.apply();
    let Job::Call {
        generation,
        origin,
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("startup new must create one shell");
    };
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

    let creation = CreateShellResult {
        address: TerminalAddress {
            hub_id: fixture.hub,
            terminal_id: TerminalId::new(),
        },
        creation_capability: SecretCapability::generate(),
    };
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request: ControlRequest {
                validity: None,
                generation,
                destination: origin,
                command: Command::Create { request },
            },
            result: Ok(Reply::Created { creation }),
            connected: true,
        }),
    );
    let session = summary_with(creation.address, RuntimeStatus::Running);
    fixture
        .state
        .terminals
        .insert(creation.address, session.clone());
    crate::initial_control::catalog(&mut fixture.state, &fixture.jobs);
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);

    let grant = super::fixtures::grant(&session);
    fixture.state.record_grant(grant, Instant::now());
    crate::terminal_input::flush_pregrant(
        &mut fixture.state,
        &fixture.jobs,
        creation.address,
        grant,
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    crate::terminal_input::sync(&mut fixture.state);
    assert!(
        !fixture
            .state
            .inputs
            .interrupted
            .contains_key(&creation.address),
        "a normal startup input receipt must not pause the stream: {:?}",
        fixture.state.inputs.interrupted
    );
    assert_eq!(take_inputs(&mut fixture.incoming), [b"y".to_vec()]);
}
