use super::{
    query_support::Fixture, terminal_input_navigation_support::PREFIX,
    terminal_input_ordering::queued,
};
use crate::{constants::READ_ONLY_LABEL, terminal_input, terminal_input_events};
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{InputFields, InputRequest};

#[test]
fn prepared_input_cannot_be_admitted_after_its_connection_or_writer_is_retired() {
    for disconnect in [false, true] {
        let mut fixture = Fixture::new();
        let input = InputRequest::new(InputFields {
            request_id: uuid::Uuid::new_v4(),
            grant: fixture.grant,
            data: PREFIX.to_vec(),
        })
        .unwrap();
        let status = fixture.state.status.clone();
        if disconnect {
            fixture.state.disconnected();
        } else {
            fixture.state.retire_writer(fixture.target.address);
        }

        // Exercise the admission boundary with a previously prepared request;
        // the keyboard caller's earlier grant lookup is not admission itself.
        assert!(!terminal_input::enqueue(
            &mut fixture.state,
            &fixture.jobs,
            input,
        ));

        assert_eq!(
            fixture.state.status,
            if disconnect { READ_ONLY_LABEL } else { &status },
        );
        assert!(fixture.state.inputs.pending.is_empty());
        assert!(fixture.state.inputs.streams.is_empty());
        assert!(fixture.state.inputs.scope.is_none());
        assert!(!fixture.state.inputs.waiting());
        fixture.no_jobs();
    }
}

#[test]
fn another_command_family_cannot_consume_an_input_completion() {
    let mut fixture = Fixture::new();
    let input = queued(&mut fixture, PREFIX);
    let unrelated = ControlRequest {
        command: Command::Renew {
            grant: fixture.grant,
        },
        ..input.clone()
    };
    let pending = fixture.state.inputs.pending.len();
    let status = fixture.state.status.clone();

    assert_eq!(
        terminal_input_events::finished(
            &mut fixture.state,
            &unrelated,
            Some(&Ok(Reply::Completed {})),
        ),
        None,
    );

    assert_eq!(fixture.state.inputs.pending.len(), pending);
    assert_eq!(fixture.state.status, status);
    assert!(input.validity.as_ref().unwrap().is_current());
    assert_eq!(
        fixture.state.writers[&fixture.target.address].grant,
        fixture.grant
    );
    fixture.finish(input, Ok(Reply::Input { duplicate: false }), true);
    assert!(fixture.state.inputs.pending.is_empty());
    fixture.no_jobs();
}
