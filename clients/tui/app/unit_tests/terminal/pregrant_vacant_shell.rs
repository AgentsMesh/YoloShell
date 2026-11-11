fn created_vacant_shell(
    status: RuntimeStatus,
    typed: &[u8],
) -> (
    super::creation_support::Fixture,
    CreateShellResult,
    TerminalSummary,
) {
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
    assert_eq!(fixture.state.focused_terminal(), None);
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(typed.to_vec()),
    ));
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
    let session = summary_with(creation.address, status);
    fixture
        .state
        .terminals
        .insert(creation.address, session.clone());
    (fixture, creation, session)
}

#[test]
fn bytes_typed_into_the_vacant_pane_wait_for_the_created_address_and_grant() {
    let (mut fixture, creation, session) = created_vacant_shell(RuntimeStatus::Running, b"x");
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
    assert_eq!(take_inputs(&mut fixture.incoming), [b"x".to_vec()]);
}

#[test]
fn bytes_typed_into_a_new_split_pane_wait_for_its_created_grant() {
    let mut fixture = super::creation_support::Fixture::new();
    fixture.state.mode = Mode::Terminal;
    assert!(crate::tabs::creation::create(
        &mut fixture.state,
        &fixture.jobs,
        fixture.origin,
        "/tmp".into(),
        None,
        crate::tabs::creation::Placement::Split(yolo_presentation_layout::Axis::Horizontal),
    ));
    let Job::Call {
        generation,
        origin,
        command: Command::Create { request },
        ..
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("a split shell creation must be dispatched");
    };
    assert_eq!(fixture.state.focused_terminal(), None);
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"x".to_vec()),
    ));
    assert_eq!(fixture.state.inputs.pregrant.len(), 1);
    assert!(matches!(
        fixture.state.inputs.pregrant[0].target,
        crate::terminal_input::PregrantTarget::Creation(_)
    ));

    let creation = fixture.created();
    let tab = fixture.state.tabs.active().id;
    let pane = fixture.state.tab_layout().focused();
    fixture.finish(
        ControlRequest {
            validity: None,
            generation,
            destination: origin,
            command: Command::Create { request },
        },
        creation,
    );
    assert!(
        fixture.state.initial_control.is_some(),
        "a bound split creation must arm initial control"
    );
    let intent = fixture.state.initial_control.as_ref().unwrap();
    assert_eq!(fixture.state.tabs.active().id, tab, "active tab changed");
    assert_eq!(fixture.state.tab_layout().focused(), pane, "focus changed");
    assert_eq!(intent.pane, pane, "intent pane changed");
    assert_eq!(fixture.state.ui_epoch, intent.ui_epoch, "epoch changed");
    assert_eq!(
        fixture.state.control_generation,
        Some(intent.generation),
        "generation changed"
    );
    assert_eq!(
        fixture
            .state
            .tab_layout()
            .panes()
            .into_iter()
            .find(|candidate| candidate.id == intent.pane)
            .and_then(|candidate| candidate.terminal),
        Some(creation.address),
        "created address did not bind the intent pane"
    );
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
        panic!("a bound split creation must acquire its terminal");
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
