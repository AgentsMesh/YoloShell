use crate::{State, query_state::same_writer, state::Writer};
use std::time::{Duration, Instant};
use yolo_hub_identity::TerminalAddress;
use yolo_hub_terminal_protocol::LeaseGrant;

impl State {
    pub fn record_grant(&mut self, grant: LeaseGrant, received: Instant) {
        if !self.client_capability_enabled(
            yolo_builtin_client_contract::ClientCapability::TerminalControl,
        ) {
            return;
        }
        crate::terminal_input_stream::granted(self, grant, received);
        let address = grant.fields().target.address;
        let writer_changed = self
            .writers
            .get(&address)
            .is_some_and(|writer| !same_writer(writer.grant, grant));
        if writer_changed {
            crate::terminal_input::clear_deferred_for(self, address);
        }
        self.control.adopt(grant.fields().target, received);
        if self
            .writers
            .get(&address)
            .is_none_or(|writer| !same_writer(writer.grant, grant) || writer.expires <= received)
        {
            crate::resize::retire(self, address);
            self.retire_query_scope(address);
        }
        self.writers.insert(
            address,
            Writer {
                grant,
                expires: received + Duration::from_millis(grant.fields().ttl_ms),
                renew: received + Duration::from_millis(grant.fields().renew_after_ms),
            },
        );
        if let Some(scope) = self
            .inputs
            .scope
            .as_ref()
            .filter(|scope| same_writer(scope.grant, grant))
        {
            scope
                .queue
                .renew_authority(received + Duration::from_millis(grant.fields().ttl_ms));
        }
        self.sync_query_authority();
    }

    pub fn grant(&self, address: TerminalAddress, now: Instant) -> Option<LeaseGrant> {
        if !self.client_capability_enabled(
            yolo_builtin_client_contract::ClientCapability::TerminalControl,
        ) {
            return None;
        }
        self.control_generation?;
        let writer = self.writers.get(&address)?;
        (writer.expires > now).then_some(writer.grant)
    }

    pub fn disconnected(&mut self) {
        crate::initial_control::disconnected(self);
        // Independently admitted creations settle by their own request identity.
        // They contain view intent, never writer authority; failures retain
        // their durable original request for read-only recovery.
        self.interrupts.disconnected();
        crate::chat::disconnected(self);
        self.control_generation = None;
        self.recoverable_status = None;
        self.client_capabilities = None;
        self.client_startup_failure = None;
        let _ = self.creations.recovery.prepare(None, Instant::now());
        self.disconnect_runtime();
        crate::terminal_input::clear_deferred(self);
        // Pregrant bytes belong to one connection's acquisition episodes.
        crate::terminal_input::clear_pregrant(self);
        self.control.disconnected(Instant::now());
        self.sync_query_authority();
    }

    pub(crate) fn retire_matching_writer(&mut self, grant: LeaseGrant) -> bool {
        let address = grant.fields().target.address;
        if self
            .writers
            .get(&address)
            .is_some_and(|writer| same_writer(writer.grant, grant))
        {
            self.retire_writer(address);
            true
        } else {
            false
        }
    }
}
