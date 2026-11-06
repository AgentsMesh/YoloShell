use std::collections::BTreeMap;
use uuid::Uuid;
use yolo_hub_identity::{HubId, TerminalAddress};
use yolo_hub_terminal_protocol::{CreateShellRequest, CreateShellResult};
use yolo_tui_creation_journal::CreationIntent;
use yolo_tui_creation_recovery::CreationRecovery;

pub(crate) struct PendingCreation {
    pub request: CreateShellRequest,
    pub origin: HubId,
    pub tab: Uuid,
    pub pane: Uuid,
    pub expected: Option<TerminalAddress>,
    pub binding_active: bool,
    pub confirmed: Option<CreateShellResult>,
    pub confirmation_retry_at: Option<std::time::Instant>,
    pub generation: Option<Uuid>,
    pub deadline: std::time::Instant,
    pub presentation: crate::creation_presentation::CreationPresentation,
    pub ui_epoch: u64,
    pub reservation: Option<crate::creation_reservation::Reservation>,
}

#[derive(Default)]
pub(crate) struct Creations {
    pub pending: BTreeMap<Uuid, PendingCreation>,
    pub finalizing: BTreeMap<Uuid, crate::finalization_state::Finalization>,
    pub journal: Option<yolo_tui_creation_journal::CreationJournal>,
    pub recovery: CreationRecovery,
    // Live, correlated proof of no creation: local non-dispatch or an explicit
    // origin pre-creation rejection. Restarted disk state cannot prove either.
    pub uncreated: BTreeMap<Uuid, CreationIntent>,
    pub persistence_error: Option<&'static str>,
    pub persistence_recover_at: Option<std::time::Instant>,
}

impl Creations {
    pub fn len(&self) -> usize {
        self.pending.len() + self.finalizing.len() + self.recovery.len() + self.uncreated.len()
    }

    pub fn retire_binding(&mut self, tab: Uuid, pane: Uuid) {
        for pending in self.pending.values_mut() {
            if pending.tab == tab && pending.pane == pane {
                pending.binding_active = false;
            }
        }
    }

    pub fn retire(&mut self, request: &CreateShellRequest) {
        self.retire_at(request, std::time::Instant::now());
    }

    pub fn retire_at(&mut self, request: &CreateShellRequest, now: std::time::Instant) {
        let id = request.fields().request_id;
        if self
            .pending
            .get(&id)
            .is_some_and(|pending| pending.request == *request && pending.confirmed.is_none())
        {
            let pending = &self.pending[&id];
            self.recovery
                .retain(
                    CreationIntent {
                        origin: pending.origin,
                        request: pending.request.clone(),
                    },
                    now,
                )
                .expect(crate::creation_constants::PENDING_CREATION_INVARIANT);
            self.pending.remove(&id);
        }
    }

    pub fn cancel_uncreated(&mut self, request: &CreateShellRequest) {
        let id = request.fields().request_id;
        if self
            .pending
            .get(&id)
            .is_some_and(|pending| pending.request == *request && pending.confirmed.is_none())
        {
            let pending = self
                .pending
                .remove(&id)
                .expect(crate::creation_constants::PENDING_CREATION_INVARIANT);
            self.uncreated.insert(
                request.fields().creation_token,
                CreationIntent {
                    origin: pending.origin,
                    request: pending.request,
                },
            );
        }
    }
}
