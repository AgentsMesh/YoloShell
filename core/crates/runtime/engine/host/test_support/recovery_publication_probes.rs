impl Host {
    #[cfg(test)]
    pub(crate) fn notify_recovery_publication(&self, manifest: &TerminalManifest) {
        let published = {
            let mut slot = lock_recover(&self.catalog_test_probes.recovery_publication);
            if slot.as_ref().is_some_and(|probe| probe.matches(manifest)) {
                slot.take().map(|probe| probe.published)
            } else {
                None
            }
        };
        if let Some(published) = published {
            let _ = published.send(());
        }
    }
}

#[cfg(test)]
pub(crate) struct ReconcileScanProbe {
    started: Option<tokio::sync::oneshot::Sender<()>>,
    repair: Option<CatalogRepairProbe>,
}

#[cfg(test)]
struct CatalogRepairProbe {
    reached: std::sync::mpsc::Sender<()>,
    release: std::sync::mpsc::Receiver<()>,
    completed: std::sync::mpsc::Sender<CatalogRepairProbeOutcome>,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CatalogRepairProbeOutcome {
    Applied,
    Superseded,
    Fenced,
    Failed,
}

#[cfg(test)]
impl ReconcileScanProbe {
    pub(crate) fn started(sender: tokio::sync::oneshot::Sender<()>) -> Self {
        Self {
            started: Some(sender),
            repair: None,
        }
    }

    pub(crate) fn repair(
        reached: std::sync::mpsc::Sender<()>,
        release: std::sync::mpsc::Receiver<()>,
        completed: std::sync::mpsc::Sender<CatalogRepairProbeOutcome>,
    ) -> Self {
        Self {
            started: None,
            repair: Some(CatalogRepairProbe {
                reached,
                release,
                completed,
            }),
        }
    }

    pub(crate) fn scan_started(&mut self) {
        if let Some(sender) = self.started.take() {
            let _ = sender.send(());
        }
    }

    pub(crate) fn before_repair(&mut self) {
        if let Some(repair) = self.repair.as_ref() {
            let _ = repair.reached.send(());
            let _ = repair.release.recv();
        }
    }

    pub(crate) fn repair_completed(&mut self, outcome: CatalogRepairProbeOutcome) {
        if let Some(repair) = self.repair.take() {
            let _ = repair.completed.send(outcome);
        }
    }
}

#[cfg(test)]
pub(crate) struct RecoveryPublicationProbe {
    terminal_id: Uuid,
    state: RecoveryState,
    published: tokio::sync::oneshot::Sender<()>,
}

#[cfg(test)]
impl RecoveryPublicationProbe {
    pub(crate) fn new(
        terminal_id: Uuid,
        state: RecoveryState,
        published: tokio::sync::oneshot::Sender<()>,
    ) -> Self {
        Self {
            terminal_id,
            state,
            published,
        }
    }

    pub(crate) fn matches(&self, manifest: &TerminalManifest) -> bool {
        self.terminal_id == manifest.terminal_id && self.state == manifest.recovery_state
    }
}
