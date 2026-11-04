use crate::State;
use std::{collections::BTreeMap, time::Instant};
use uuid::Uuid;
use yolo_hub_client_channels::{ControlRequest, RequestScope, RequestValidity};
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};
use yolo_hub_rpc_protocol::Command;
use yolo_hub_terminal_protocol::{InputRequest, LeaseGrant};
use yolo_tui_terminal::QueryScope;

pub(crate) struct ActiveQueryScope {
    pub parser: QueryScope,
    pub queue: RequestScope,
    pub grant: LeaseGrant,
    pub observer: Uuid,
    pub control: Uuid,
}

#[derive(Default)]
pub(crate) struct Queries {
    pub scopes: BTreeMap<TerminalAddress, ActiveQueryScope>,
    pub pending: BTreeMap<Uuid, PendingReply>,
}

pub(crate) struct PendingReply {
    pub request: InputRequest,
    pub generation: Uuid,
    pub validity: RequestValidity,
}

impl PendingReply {
    pub fn matches(&self, received: &ControlRequest) -> bool {
        let Command::Input { request } = &received.command else {
            return false;
        };
        request == &self.request
            && received.generation == Some(self.generation)
            && received.destination == self.request.fields().grant.fields().target.address.hub_id
            && received.validity.as_ref().is_some_and(|validity| {
                validity.same_scope(&self.validity)
                    && validity.deadline() == self.validity.deadline()
            })
    }
}

pub(crate) fn same_writer(left: LeaseGrant, right: LeaseGrant) -> bool {
    let (left, right) = (left.fields(), right.fields());
    left.target == right.target
        && left.principal == right.principal
        && left.generation == right.generation
        && left.capability == right.capability
}

impl State {
    pub(crate) fn query_grant(&self, target: RuntimeTarget) -> Option<LeaseGrant> {
        self.query_grant_bound(target, self.bound_terminals().contains(&target.address))
    }

    fn query_grant_bound(&self, target: RuntimeTarget, bound: bool) -> Option<LeaseGrant> {
        if !crate::observation_events::current(self, target)
            || !self.input_ready(target.address)
            || self.is_offline(target.address)
            || self.ended_epoch(target.address) == Some(target.runtime_epoch)
            || !bound
        {
            return None;
        }
        let grant = self.grant(target.address, Instant::now())?;
        (grant.fields().target == target && grant.fields().principal == self.principal)
            .then_some(grant)
    }

    pub(crate) fn retire_query_scope(&mut self, address: TerminalAddress) {
        self.queries.scopes.remove(&address);
        if let Some(surface) = self.surfaces.get_mut(&address) {
            surface.reset_query_scope();
        }
        self.queries.pending.retain(|_, pending| {
            if pending.request.fields().grant.fields().target.address == address {
                pending.validity.reject();
                false
            } else {
                true
            }
        });
    }

    pub(crate) fn sync_query_authority(&mut self) {
        crate::terminal_input::sync(self);
        if self.queries.scopes.is_empty() {
            return;
        }
        let bound = self.bound_terminals();
        let retired = self
            .queries
            .scopes
            .iter()
            .filter_map(|(address, scope)| {
                let valid = self
                    .query_grant_bound(scope.grant.fields().target, bound.contains(address))
                    .is_some_and(|grant| same_writer(grant, scope.grant))
                    && self.observer_generation(*address) == Some(scope.observer)
                    && self.control_generation == Some(scope.control);
                (!valid).then_some(*address)
            })
            .collect::<Vec<_>>();
        for address in retired {
            self.retire_query_scope(address);
        }
    }

    pub(crate) fn retire_writer(
        &mut self,
        address: TerminalAddress,
    ) -> Option<crate::state::Writer> {
        crate::resize::retire(self, address);
        self.inputs.retire_writer(address);
        crate::terminal_input::clear_deferred_for(self, address);
        self.retire_query_scope(address);
        self.control.lost(address, Instant::now());
        self.writers.remove(&address)
    }
}
