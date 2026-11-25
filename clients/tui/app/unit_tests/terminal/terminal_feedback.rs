use super::{fixtures, query_support};
use crate::{channel_events, network::Job};
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{
    LeaseMode, LeaseRequest, PublicError, RuntimeStatus, TerminalSummary,
};

fn conflict() -> ControlError {
    ControlError::Client(yolo_hub_client::ClientError::Remote(PublicError::Conflict))
}

#[test]
fn blocked_runtime_conflict_explains_terminal_state_instead_of_claiming_another_writer() {
    let mut fixture = query_support::Fixture::new();
    let mut fields = fixture.state.terminals[&fixture.target.address]
        .fields()
        .clone();
    fields.runtime.as_mut().unwrap().status = RuntimeStatus::OutputBlocked;
    fixture
        .state
        .replace_terminals(vec![TerminalSummary::new(fields).unwrap()]);
    let command = Command::Lease {
        request: LeaseRequest {
            request_id: uuid::Uuid::new_v4(),
            target: fixture.target,
            principal: fixture.grant.fields().principal,
            mode: LeaseMode::Acquire {},
        },
    };
    let generation = fixture.state.control_generation;
    channel_events::apply(
        &mut fixture.state,
        &fixture.jobs,
        ControlEvent::Finished {
            request: ControlRequest {
                validity: None,
                generation,
                destination: fixture.target.address.hub_id,
                command,
            },
            result: Err(conflict()),
            connected: true,
        },
    );
    assert_eq!(
        fixture.state.status,
        yolo_tui_lifecycle::OUTPUT_BLOCKED_HINT
    );
    fixture.no_jobs();
}

#[test]
fn a_previous_pty_incarnations_error_cannot_revoke_its_successors_writer_or_feedback() {
    let mut fixture = query_support::Fixture::new();
    let old = fixture.grant;
    let mut fields = fixture.state.terminals[&fixture.target.address]
        .fields()
        .clone();
    fields.runtime.as_mut().unwrap().target.runtime_epoch = yolo_hub_identity::RuntimeEpoch::new();
    let successor = TerminalSummary::new(fields).unwrap();
    let grant = fixtures::grant(&successor);
    fixture.state.replace_terminals(vec![successor]);
    fixture.state.record_grant(grant, std::time::Instant::now());
    let before = fixture.state.status.clone();
    let generation = fixture.state.control_generation;
    channel_events::apply(
        &mut fixture.state,
        &fixture.jobs,
        ControlEvent::Finished {
            request: ControlRequest {
                validity: None,
                generation,
                destination: fixture.target.address.hub_id,
                command: Command::Renew { grant: old },
            },
            result: Err(conflict()),
            connected: true,
        },
    );
    assert_eq!(fixture.state.status, before);
    assert_eq!(fixture.state.writers[&fixture.target.address].grant, grant);
    assert!(!matches!(fixture.incoming.try_recv(), Ok(Job::Call { .. })));
}
