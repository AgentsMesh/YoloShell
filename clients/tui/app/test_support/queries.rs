use super::fixtures;
use crate::{
    State,
    constants::QUEUE_CAPACITY,
    events,
    network::{Job, ObservationUpdate, Update},
};
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;
use yolo_hub_client_channels::{ControlEvent, ControlRequest};
use yolo_hub_identity::RuntimeTarget;
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::*;
use yolo_tui_terminal::TerminalSurface;

#[path = "query_constants.rs"]
mod constants;
pub use constants::*;

pub struct Fixture {
    pub state: State,
    pub target: RuntimeTarget,
    pub observer: Uuid,
    pub grant: LeaseGrant,
    pub jobs: mpsc::Sender<Job>,
    pub incoming: mpsc::Receiver<Job>,
}

impl Fixture {
    pub fn new() -> Self {
        Self::with_capacity(QUEUE_CAPACITY)
    }
    pub fn with_capacity(capacity: usize) -> Self {
        let session = fixtures::session();
        let target = session.fields().runtime.unwrap().target;
        let grant = fixtures::grant(&session);
        let mut state = crate::client_capability_support::state();
        state.control_generation = Some(Uuid::new_v4());
        state.principal = grant.fields().principal;
        state.replace_terminals(vec![session]);
        state.bind_focused(Some(target.address)).unwrap();
        state.record_grant(grant, Instant::now());
        state.set_last_size(target.address, TerminalSize::default());
        state.surfaces.insert(
            target.address,
            TerminalSurface::new(&snapshot(target, INITIAL_SEQUENCE, POSITION)),
        );
        let observer = Uuid::new_v4();
        state.set_observer_generation(target.address, observer);
        let (jobs, incoming) = mpsc::channel(capacity);
        Self {
            state,
            target,
            observer,
            grant,
            jobs,
            incoming,
        }
    }
    pub fn event(&self, bytes: &[u8]) -> TerminalEvent {
        let sequence = self
            .state
            .surfaces
            .get(&self.target.address)
            .map_or(INITIAL_SEQUENCE, |surface| surface.cursor().after_sequence)
            + FIRST_OUTPUT_SEQUENCE;
        TerminalEvent::Output {
            chunk: OutputChunk::new(OutputChunkFields {
                target: self.target,
                sequence,
                data: bytes.to_vec(),
            })
            .unwrap(),
        }
    }
    pub fn emit(&mut self, bytes: &[u8]) {
        self.scoped(ObservationUpdate::Terminal {
            event: self.event(bytes),
        });
    }
    pub fn scoped(&mut self, update: ObservationUpdate) {
        events::apply(
            &mut self.state,
            &self.jobs,
            Update::Observation {
                target: self.target,
                generation: self.observer,
                update,
            },
        );
    }
    pub fn queued(&mut self) -> ControlRequest {
        let Job::TerminalReply {
            generation,
            request,
            validity,
        } = self.incoming.try_recv().unwrap()
        else {
            panic!()
        };
        ControlRequest {
            generation: Some(generation),
            destination: request.fields().grant.fields().target.address.hub_id,
            command: Command::Input { request },
            validity: Some(validity),
        }
    }
    pub fn no_jobs(&mut self) {
        assert!(self.incoming.try_recv().is_err());
    }
    pub fn ready_size(&mut self) {
        self.state
            .set_last_size(self.target.address, TerminalSize::default());
    }
    pub fn replace_writer(&mut self) -> LeaseGrant {
        let mut fields = *self.grant.fields();
        fields.generation += STEP;
        fields.capability = SecretCapability::generate();
        let grant = LeaseGrant::new(fields).unwrap();
        self.state.record_grant(grant, Instant::now());
        self.ready_size();
        grant
    }
    pub fn finish(
        &mut self,
        request: ControlRequest,
        result: Result<yolo_hub_rpc_protocol::Reply, yolo_hub_client_channels::ControlError>,
        connected: bool,
    ) {
        events::apply(
            &mut self.state,
            &self.jobs,
            Update::Control(ControlEvent::Finished {
                request,
                result,
                connected,
            }),
        );
    }
}

pub fn input(request: &ControlRequest) -> &InputRequest {
    let Command::Input { request } = &request.command else {
        panic!()
    };
    request
}

pub fn snapshot(target: RuntimeTarget, sequence: u64, data: &[u8]) -> TerminalSnapshot {
    TerminalSnapshot::new(TerminalSnapshotFields {
        target,
        barrier_sequence: sequence,
        size: TerminalSize::default(),
        data: data.to_vec(),
    })
    .unwrap()
}
