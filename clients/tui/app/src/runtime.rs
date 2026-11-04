use crate::State;
use yolo_hub_identity::{RuntimeTarget, TerminalAddress};

/// Owns the lifecycle transitions of one client-side terminal runtime.
///
/// Per-terminal bookkeeping lives in one aggregate (`terminal_runtime`), so
/// every transition that ends, replaces, detaches or disconnects a runtime has
/// one removal target and a new fact cannot escape cleanup. Presentation
/// admission and liveness live in `tab_binding`/`tab_liveness`; this module only
/// mutates the runtime facts.
impl State {
    /// A live observation proved this exact runtime incarnation exited.
    pub(crate) fn end_runtime(&mut self, target: RuntimeTarget) {
        self.mark_ended(target.address, target.runtime_epoch);
        self.clear_offline(target.address);
        self.retire_writer(target.address);
    }

    /// The catalogue moved the terminal to a different runtime target, or the
    /// record disappeared. The old incarnation's transport and presentation
    /// state must not leak into the replacement.
    pub(crate) fn retire_runtime(&mut self, address: TerminalAddress) {
        if let Some(writer) = self.retire_writer(address) {
            self.retired_writers.push(writer);
        }
        self.inputs.discard_writer(address);
        self.control.retire(address);
        self.terminal_runtime.remove(&address);
        self.surfaces.remove(&address);
    }

    /// Releases state owned by a pane presentation when that pane stops showing
    /// the terminal. The catalogue record, ended epoch and writer/control
    /// authority have their own lifetime transitions.
    pub(crate) fn detach_binding(&mut self, address: TerminalAddress) {
        self.surfaces.remove(&address);
        self.clear_detached_binding_facts(address);
        self.inputs.snapshot_pending.remove(&address);
        self.inputs.overflowed.remove(&address);
        self.inputs
            .pregrant
            .retain(|item| item.target != crate::terminal_input::PregrantTarget::Address(address));
    }

    /// Drops connection-scoped runtime state while retaining retained screens
    /// and the catalogue history for reconnection.
    pub(crate) fn disconnect_runtime(&mut self) {
        self.writers.clear();
        self.clear_connection_runtime_facts();
        self.inputs.snapshot_pending.clear();
    }
}
