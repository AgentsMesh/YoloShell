use crate::interrupt_constants::*;
use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_agent_protocol::{AgentControlRequest, FailureCode};
use yolo_hub_client_channels::ControlRequest;
use yolo_hub_identity::{AgentAddress, AgentTarget, ExecutionId};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::LeaseGrant;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Phase {
    Sending,
    Accepted,
    Unknown,
    Sent,
    NotSent,
    Rejected(FailureCode),
}
impl Phase {
    pub fn blocks(self) -> bool {
        matches!(self, Self::Sending | Self::Accepted | Self::Unknown)
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Sending => SENDING,
            Self::Accepted => ACCEPTED,
            Self::Unknown => UNKNOWN,
            Self::Sent => SENT,
            Self::NotSent => NOT_SENT,
            Self::Rejected(FailureCode::PermissionDenied) => PERMISSION_DENIED,
            Self::Rejected(FailureCode::PreconditionsChanged) => STALE,
            Self::Rejected(FailureCode::Unsupported) => UNAVAILABLE,
            Self::Rejected(FailureCode::Expired) => EXPIRED,
            Self::Rejected(FailureCode::AdapterRejected) => REJECTED,
        }
    }
}

pub(crate) struct PendingInterrupt {
    pub request: AgentControlRequest,
    pub grant: LeaseGrant,
    pub generation: Uuid,
    pub phase: Phase,
}
#[derive(Default)]
pub(crate) struct Interrupts {
    entries: BTreeMap<(AgentAddress, ExecutionId), PendingInterrupt>,
}
impl Interrupts {
    pub fn get(&self, target: AgentTarget) -> Option<&PendingInterrupt> {
        self.entries.get(&(target.address, target.execution_id))
    }
    pub fn dismiss_completed(&mut self, target: AgentTarget) {
        let key = (target.address, target.execution_id);
        if self
            .entries
            .get(&key)
            .is_some_and(|entry| !entry.phase.blocks())
        {
            self.entries.remove(&key);
        }
    }
    pub fn has_capacity(&mut self) -> bool {
        self.entries.retain(|_, entry| entry.phase.blocks());
        self.entries.len() < MAX_TRACKED
    }
    pub fn insert(&mut self, request: AgentControlRequest, grant: LeaseGrant, generation: Uuid) {
        let target = request.fields().target;
        self.entries.insert(
            (target.address, target.execution_id),
            PendingInterrupt {
                request,
                grant,
                generation,
                phase: Phase::Sending,
            },
        );
    }
    pub fn correlated(&mut self, incoming: &ControlRequest) -> Option<&mut PendingInterrupt> {
        let Command::AgentControl { request, grant } = &incoming.command else {
            return None;
        };
        let target = request.fields().target;
        let entry = self
            .entries
            .get_mut(&(target.address, target.execution_id))?;
        (incoming.generation == Some(entry.generation)
            && incoming.destination == target.address.hub_id
            && entry.request == *request
            && entry.grant == *grant
            && entry.phase.blocks())
        .then_some(entry)
    }
    pub fn disconnected(&mut self) {
        for entry in self.entries.values_mut() {
            if matches!(entry.phase, Phase::Sending | Phase::Accepted) {
                entry.phase = Phase::Unknown;
            }
        }
    }
}
