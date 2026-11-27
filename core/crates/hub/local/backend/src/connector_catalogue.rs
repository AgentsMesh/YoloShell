use crate::{LocalBackend, connector_admission::admission};
use std::collections::BTreeMap;
use yolo_hub_connector_registry::{Catalogue, RuntimeAdmission};
use yolo_hub_identity::TerminalAddress;
use yolo_terminal_model::TerminalView;

#[derive(Clone, Copy)]
pub(crate) enum ObservationScope {
    Complete,
    Terminal(TerminalAddress),
}

impl LocalBackend {
    // Callers hold the native projection lock (when used) and observation gate
    // continuously from before their Host read through this reconciliation.
    pub(crate) async fn connector_catalogue(
        &self,
        views: Option<&[TerminalView]>,
        scope: ObservationScope,
    ) -> Catalogue {
        let Some(registry) = &self.connectors else {
            return Catalogue::default();
        };
        let mut registry = registry.lock().await;
        let mut facts = BTreeMap::new();
        for target in registry.terminals() {
            if matches!(scope, ObservationScope::Terminal(address) if target.address != address) {
                continue;
            }
            let fact = views
                .and_then(|views| {
                    views.iter().find(|view| {
                        view.manifest.terminal_id == target.address.terminal_id.as_uuid()
                    })
                })
                .map_or(RuntimeAdmission::Unavailable, |view| {
                    admission(target, view)
                });
            facts.insert(target, fact);
        }
        let reconciled = match scope {
            ObservationScope::Complete => registry.reconcile(&facts),
            ObservationScope::Terminal(_) => registry.reconcile_observed(&facts),
        };
        match reconciled.and_then(|()| registry.catalogue()) {
            Ok(catalogue) => catalogue,
            Err(_) => {
                let mut historical = registry.last_known_catalogue();
                // Storage failure cannot confirm that an enrollment remains enabled.
                historical.primaries.clear();
                historical
            }
        }
    }
}
