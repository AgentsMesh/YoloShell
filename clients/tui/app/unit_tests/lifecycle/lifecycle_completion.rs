use super::fixtures;
use crate::{
    State, events,
    network::{Job, Update},
};
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_catalog_client::Catalog;
use yolo_hub_client_channels::{CatalogEvent, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{
    RuntimeStatus, TerminalAction, TerminalCommand, TerminalSelection, TerminalSummary,
};

#[test]
fn delayed_resume_reply_cannot_overwrite_catalog_observation_after_termination() {
    let running = fixtures::session();
    let address = running.fields().address;
    let mut fields = running.fields().clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::Exited;
    let exited = TerminalSummary::new(fields).unwrap();
    let generation = Uuid::new_v4();
    let mut state = crate::client_capability_support::state();
    state.control_generation = Some(generation);
    state.replace_terminals(vec![running.clone()]);
    let (jobs, mut incoming) = mpsc::channel(crate::constants::QUEUE_CAPACITY);
    complete(
        &mut state,
        &jobs,
        TerminalAction::Terminate { force: false },
        exited.clone(),
    );
    let Job::Refresh { request_id } = incoming.try_recv().unwrap() else {
        panic!()
    };
    events::apply(
        &mut state,
        &jobs,
        Update::Catalog(CatalogEvent {
            request_id,
            result: Ok(Catalog {
                terminals: vec![exited.clone()],
                ..Catalog::default()
            }),
        }),
    );
    assert_eq!(state.terminals[&address], exited);
    assert!(state.inventory.current);
    complete(&mut state, &jobs, TerminalAction::Resume {}, running);
    assert_eq!(state.terminals[&address], exited);
    assert!(!state.inventory.current);
    assert!(
        matches!(incoming.try_recv().unwrap(), Job::Refresh { request_id } if Some(request_id) == state.catalog_pending)
    );
    assert!(incoming.try_recv().is_err());
    assert_eq!(state.control_generation, Some(generation));
    assert!(state.writers.is_empty());
}

fn complete(
    state: &mut State,
    jobs: &mpsc::Sender<Job>,
    action: TerminalAction,
    session: TerminalSummary,
) {
    let selection = TerminalSelection {
        address: session.fields().address,
        expected_runtime: session
            .fields()
            .runtime
            .map(|runtime| runtime.target.runtime_epoch),
    };
    let generation = state.control_generation;
    events::apply(
        state,
        jobs,
        Update::Control(ControlEvent::Finished {
            request: ControlRequest {
                validity: None,
                generation,
                destination: selection.address.hub_id,
                command: Command::Terminal {
                    command: TerminalCommand {
                        request_id: Uuid::new_v4(),
                        selection,
                        action,
                    },
                },
            },
            result: Ok(Reply::Terminal { terminal: session }),
            connected: true,
        }),
    );
}
