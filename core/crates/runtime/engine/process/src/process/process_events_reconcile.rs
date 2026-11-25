use super::ProcessEvents;
use crate::process::LiveProcessIdentity;
use std::time::Instant;

pub(in crate::process) struct EventReconciliationDebt {
    pending: bool,
    requires_followup: bool,
}

impl ProcessEvents {
    #[must_use]
    pub(in crate::process) fn capture_reconciliation_debt(
        &self,
    ) -> Option<EventReconciliationDebt> {
        (self.pending_reconciliation || self.reconciliation_requires_followup).then_some(
            EventReconciliationDebt {
                pending: self.pending_reconciliation,
                requires_followup: self.reconciliation_requires_followup,
            },
        )
    }

    pub(in crate::process) fn merge_reconciliation_debt(
        &mut self,
        debt: Option<EventReconciliationDebt>,
    ) {
        let Some(debt) = debt else {
            return;
        };
        self.pending_reconciliation |= debt.pending;
        self.reconciliation_requires_followup |= debt.requires_followup;
        self.invalidate_inventory_publication();
    }

    pub(in crate::process) fn reconcile_inventory(
        &mut self,
        identities: impl IntoIterator<Item = LiveProcessIdentity>,
    ) -> bool {
        let epoch_available = self.begin_inventory_reconciliation();
        let admitted_debt = std::mem::take(&mut self.pending_reconciliation);
        let requires_followup =
            admitted_debt && std::mem::take(&mut self.reconciliation_requires_followup);
        self.reconciled_at = Instant::now();
        #[cfg(target_os = "macos")]
        {
            // EV_DELETE discards queued knotes, so drain before watcher removal.
            if self.gate.is_some() {
                self.drain_before_reconcile();
            }
            if let Some(gate) = self.gate.as_mut() {
                match gate.reconcile(identities) {
                    Ok(outcome) => {
                        self.pending_reconciliation |=
                            outcome.invalidated || !outcome.registered.is_empty();
                    }
                    Err(error) => {
                        super::report_failure(error);
                        self.gate = None;
                        self.pending_reconciliation = true;
                        self.reconciliation_requires_followup = false;
                    }
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = identities;
        let publication_quiescent = self.post_inventory_quiescent() && epoch_available;
        #[cfg(target_os = "macos")]
        if requires_followup && self.gate.is_none() {
            self.record_inventory_publication(false);
            return false;
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (admitted_debt, requires_followup);
        self.record_inventory_publication(publication_quiescent);
        publication_quiescent
    }

    pub(in crate::process) fn post_inventory_quiescent(&mut self) -> bool {
        self.invalidate_inventory_publication();
        let pending = self.pending_reconciliation || self.audit_is_due();
        self.pending_reconciliation = pending;
        #[cfg(target_os = "macos")]
        {
            let Some(gate) = self.gate.as_mut() else {
                return !pending;
            };
            match gate.changed() {
                Ok(changed) => {
                    self.pending_reconciliation |= changed;
                    !pending && !changed
                }
                Err(error) => {
                    super::report_failure(error);
                    self.gate = None;
                    self.pending_reconciliation = true;
                    self.reconciliation_requires_followup = false;
                    false
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            !pending
        }
    }

    pub(in crate::process) fn post_inventory_event_read_quiescent(&mut self) -> bool {
        self.invalidate_inventory_publication();
        self.pending_reconciliation |= self.audit_is_due();
        #[cfg(target_os = "macos")]
        {
            let Some(gate) = self.gate.as_mut() else {
                return false;
            };
            match gate.changed() {
                Ok(changed) => {
                    self.pending_reconciliation |= changed;
                    !changed
                }
                Err(error) => {
                    super::report_failure(error);
                    self.gate = None;
                    self.pending_reconciliation = true;
                    self.reconciliation_requires_followup = false;
                    false
                }
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            false
        }
    }

    #[cfg(target_os = "macos")]
    fn drain_before_reconcile(&mut self) {
        let Some(gate) = self.gate.as_mut() else {
            return;
        };
        match gate.changed() {
            Ok(changed) => self.pending_reconciliation |= changed,
            Err(error) => {
                super::report_failure(error);
                self.gate = None;
                self.pending_reconciliation = true;
                self.reconciliation_requires_followup = false;
            }
        }
    }
}
