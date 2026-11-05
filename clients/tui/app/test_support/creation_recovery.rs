use super::{creation_support::Fixture, finalization_support};
use crate::{
    creation_recovery, events,
    network::{Job, Update},
};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::{CreationQueryOutcome, CreationQueryResult};

pub const CHANGED_NAME: &str = "conflicting prepared shell";
pub const ONE_RECORD: usize = 1;

pub fn take_query(fixture: &mut Fixture, now: Instant) -> ControlRequest {
    creation_recovery::flush(&mut fixture.state, &fixture.jobs, now);
    let mut query = None;
    while let Ok(job) = fixture.incoming.try_recv() {
        match job {
            Job::CreationQuery(request) => {
                assert!(query.is_none());
                assert!(matches!(request.command, Command::QueryCreation { .. }));
                query = Some(request);
            }
            Job::Refresh { .. } => {}
            _ => panic!("intent recovery may only query the origin"),
        }
    }
    query.expect("expected original creation query")
}

pub fn reply(fixture: &mut Fixture, request: ControlRequest, outcome: CreationQueryOutcome) {
    let Command::QueryCreation { request: query } = &request.command else {
        panic!()
    };
    let query =
        CreationQueryResult::new(query.creation().fields().creation_token, outcome).unwrap();
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request,
            result: Ok(Reply::CreationQuery { query }),
            connected: true,
        }),
    );
}

pub fn retire(fixture: &mut Fixture, request: ControlRequest) {
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request,
            result: Err(ControlError::Client(
                yolo_hub_client::ClientError::OutcomeUnknownResponse(
                    yolo_hub_terminal_protocol::PublicError::InvalidPayload,
                ),
            )),
            connected: true,
        }),
    );
    assert!(fixture.state.creations.pending.is_empty());
    assert_eq!(fixture.state.creations.recovery.len(), ONE_RECORD);
    finalization_support::no_commands(fixture);
}

pub fn ready(fixture: &mut Fixture) {
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(crate::client_capability_support::ready(uuid::Uuid::new_v4())),
    );
}
