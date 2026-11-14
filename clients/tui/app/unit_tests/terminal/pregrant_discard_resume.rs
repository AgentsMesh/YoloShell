#[test]
fn an_interrupted_prefix_clears_only_after_its_last_pending_receipt() {
    let mut fixture = Fixture::new();
    let first = super::terminal_input_ordering::queued(&mut fixture, b"x");
    let second = super::terminal_input_ordering::queued(&mut fixture, b"y");
    let address = fixture.target.address;
    fixture
        .state
        .inputs
        .interrupted
        .insert(address, fixture.grant);
    fixture.finish(first, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.interrupted.contains_key(&address));
    fixture.finish(second, Ok(Reply::Input { duplicate: false }), true);
    assert!(!fixture.state.inputs.interrupted.contains_key(&address));
}

#[test]
fn read_only_typing_is_reported_instead_of_being_queued() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    fixture.state.retire_writer(fixture.target.address);
    fixture.state.control.observe(fixture.target);
    assert!(!terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(TYPED.to_vec()),
    ));
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert_ne!(fixture.state.status, crate::constants::INPUT_WAITING_LABEL);
    fixture.no_jobs();
}

#[test]
fn leaving_the_terminal_discards_bytes_queued_for_its_grant() {
    let (mut fixture, request) = reattaching();
    typed(&mut fixture);
    fixture.state.mode = Mode::Inventory;
    fixture.finish(
        request,
        Ok(Reply::Lease {
            grant: fixture.grant,
        }),
        true,
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn explicit_resume_flushes_only_bytes_typed_after_the_interruption() {
    let mut fixture = Fixture::new();
    fixture.state.mode = Mode::Terminal;
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"old\r".to_vec()),
    ));
    let Job::UserInput(old) = fixture.incoming.try_recv().unwrap() else {
        panic!("an established writer admits input directly");
    };
    fixture.finish(
        old,
        Err(ControlError::Client(yolo_hub_client::ClientError::Remote(
            yolo_hub_terminal_protocol::PublicError::OutcomeUnknown,
        ))),
        true,
    );
    assert!(
        fixture
            .state
            .inputs
            .interrupted
            .contains_key(&fixture.target.address)
    );
    assert!(!fixture.state.writers.contains_key(&fixture.target.address));

    crate::polling::resume_input(&mut fixture.state, &fixture.jobs, false);
    let Job::Acquire {
        generation,
        origin,
        request,
        validity,
    } = fixture.incoming.try_recv().unwrap()
    else {
        panic!("explicit resume must acquire a fresh writer");
    };
    let request = ControlRequest {
        generation: Some(generation),
        destination: origin,
        command: Command::Lease { request },
        validity: Some(validity),
    };
    assert!(terminal_actions::deliver(
        &mut fixture.state,
        &fixture.jobs,
        Ok(b"new\r".to_vec()),
    ));
    assert!(fixture.incoming.try_recv().is_err());

    // A fresh writer identity, not a renewal of the interrupted lease.
    let mut fields = *fixture.grant.fields();
    fields.generation += STEP;
    fields.capability = SecretCapability::generate();
    let fresh = LeaseGrant::new(fields).unwrap();
    fixture.finish(request, Ok(Reply::Lease { grant: fresh }), true);
    let Job::UserInput(input) = fixture.incoming.try_recv().unwrap() else {
        panic!("the resume grant must flush the new bytes");
    };
    let Command::Input { request } = input.command else {
        panic!("pregrant input must use the input command");
    };
    assert_eq!(request.fields().data, b"new\r");
    assert!(
        !fixture
            .state
            .inputs
            .interrupted
            .contains_key(&fixture.target.address)
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert!(fixture.incoming.try_recv().is_err());
}

#[test]
fn explicit_release_discards_bytes_queued_for_its_grant() {
    let (mut fixture, _request) = reattaching();
    typed(&mut fixture);
    fixture.state.prefix = true;
    crate::actions::handle(
        &mut fixture.state,
        &fixture.jobs,
        fixture.target.address.hub_id,
        Event::Key(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE)),
    );
    assert!(fixture.state.inputs.pregrant.is_empty());
    assert!(!fixture.state.control.permits(fixture.target));
}

fn summary_with(address: TerminalAddress, status: RuntimeStatus) -> TerminalSummary {
    let mut fields = super::fixtures::session().fields().clone();
    fields.address = address;
    let runtime = fields.runtime.as_mut().unwrap();
    runtime.target.address = address;
    runtime.status = status;
    TerminalSummary::new(fields).unwrap()
}

fn take_inputs(incoming: &mut tokio::sync::mpsc::Receiver<Job>) -> Vec<Vec<u8>> {
    let mut data = Vec::new();
    while let Ok(job) = incoming.try_recv() {
        if let Job::UserInput(input) = job {
            let Command::Input { request } = input.command else {
                panic!("pregrant input must use the input command");
            };
            data.push(request.fields().data.clone());
        }
    }
    data
}
