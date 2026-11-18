use super::creation_support::Fixture;
use crate::{
    events,
    network::{Job, Update},
};
use std::time::Instant;
use yolo_hub_client_channels::{ControlError, ControlEvent, ControlRequest};
use yolo_hub_rpc_protocol::{Command, Reply};
use yolo_hub_terminal_protocol::CreateShellResult;

pub fn confirmed(fixture: &mut Fixture) -> (ControlRequest, CreateShellResult, ControlRequest) {
    let create = fixture.start();
    let created = fixture.created();
    fixture.finish(create.clone(), created);
    let finalize = take(fixture);
    (create, created, finalize)
}

pub fn take(fixture: &mut Fixture) -> ControlRequest {
    let mut result = None;
    while let Ok(job) = fixture.incoming.try_recv() {
        match job {
            Job::Refresh { .. } => {}
            Job::Finalize {
                generation,
                creation_token,
                creation,
                validity,
            } => {
                assert!(
                    result.is_none(),
                    "one attempt per creation may be in flight"
                );
                result = Some(ControlRequest {
                    generation: Some(generation),
                    destination: creation.address.hub_id,
                    command: Command::Finalize {
                        creation_token,
                        creation,
                    },
                    validity: Some(validity),
                });
            }
            _ => panic!("settlement must not replay Create or issue writer/lifecycle commands"),
        }
    }
    result.expect("expected an exact Finalize attempt")
}

pub fn finish(fixture: &mut Fixture, request: ControlRequest, result: Result<Reply, ControlError>) {
    events::apply(
        &mut fixture.state,
        &fixture.jobs,
        Update::Control(ControlEvent::Finished {
            request,
            result,
            connected: true,
        }),
    );
}

pub fn token(request: &ControlRequest) -> uuid::Uuid {
    let Command::Finalize { creation_token, .. } = request.command else {
        panic!()
    };
    creation_token
}

pub fn retry(fixture: &mut Fixture, request: &ControlRequest) -> ControlRequest {
    let at = fixture.state.creations.finalizing[&token(request)].retry_at;
    crate::finalization_actions::flush(&mut fixture.state, &fixture.jobs, at);
    take(fixture)
}

pub fn no_commands(fixture: &mut Fixture) {
    while let Ok(job) = fixture.incoming.try_recv() {
        assert!(matches!(job, Job::Refresh { .. }));
    }
}

pub fn later() -> Instant {
    Instant::now() + crate::finalization_state::RETRY_DELAY
}
