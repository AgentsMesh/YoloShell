pub(crate) const CATALOG_REPAIR_NAMESPACE_PREFIX: &str = ".yolo-catalog-repair-";
pub(crate) const CATALOG_REPAIR_ENTRY_UNAVAILABLE: &str = "catalog repair target is unavailable";
pub(crate) const CATALOG_REPAIR_FENCED: &str = "catalog repair fenced by a newer host generation";
pub(crate) const CATALOG_REPAIR_SUPERSEDED: &str =
    "catalog repair superseded by a newer session row";

pub(crate) struct CatalogRepairRequest {
    expectation: crate::catalog::CatalogRowExpectation,
    manifest: TerminalManifest,
    catalog_entry: crate::catalog::CatalogRepairEntry,
    catalog: Weak<Mutex<Catalog>>,
    namespace: Weak<crate::namespace_lease::NamespaceLease>,
    source: CatalogRepairSource,
}

#[derive(Clone, Copy)]
pub(crate) enum CatalogRepairSource {
    Indexed,
    Diagnostic,
}

impl CatalogRepairSource {
    pub(crate) fn counts_diagnostic_on_failure(self) -> bool {
        matches!(self, Self::Indexed)
    }
}

pub(crate) fn repair_request(
    catalog_entry: Option<&crate::paths::private_file::PrivateFileEntry>,
    catalog: Weak<Mutex<Catalog>>,
    namespace: Weak<crate::namespace_lease::NamespaceLease>,
    expectation: crate::catalog::CatalogRowExpectation,
    manifest: TerminalManifest,
    source: CatalogRepairSource,
) -> Result<CatalogRepairRequest> {
    if let Some(namespace) = namespace.upgrade() {
        namespace.verify()?;
    }
    let entry = catalog_entry
        .ok_or_else(|| anyhow::anyhow!(CATALOG_REPAIR_ENTRY_UNAVAILABLE))?
        .reopen()?;
    let catalog_entry = Catalog::capture_repair_entry(entry)?;
    Ok(CatalogRepairRequest {
        expectation,
        manifest,
        catalog_entry,
        catalog,
        namespace,
        source,
    })
}

pub(crate) fn repair_catalog_request(
    paths: &Paths,
    request: CatalogRepairRequest,
    generation: u64,
    #[cfg(test)] probe: &mut Option<ReconcileScanProbe>,
) -> Result<Option<TerminalManifest>> {
    let CatalogRepairRequest {
        expectation,
        manifest,
        catalog_entry,
        catalog: catalog_weak,
        namespace,
        source: _,
    } = request;
    #[cfg(test)]
    if let Some(probe) = probe.as_mut() {
        probe.before_repair();
    }
    let repaired = if let Some(catalog) = catalog_weak.upgrade() {
        lock_recover(&catalog).repair_for_controller(generation, expectation, &manifest)
    } else {
        // The synthetic SQLite name resolves through the retained physical artifact registry.
        let logical_path = paths.catalog().with_file_name(format!(
            "{CATALOG_REPAIR_NAMESPACE_PREFIX}{}",
            Uuid::new_v4()
        ));
        let mut catalog = match Catalog::open_repair_entry(catalog_entry, logical_path) {
            Ok(catalog) => catalog,
            Err(error) => {
                #[cfg(test)]
                report_repair_outcome(probe, CatalogRepairProbeOutcome::Failed);
                return Err(error);
            }
        };
        catalog.repair_for_controller(generation, expectation, &manifest)
    };
    #[cfg(test)]
    report_repair_outcome(probe, repair_outcome(&repaired));
    if let Some(namespace) = namespace.upgrade() {
        namespace.verify()?;
    }
    match repaired? {
        crate::catalog::ControllerWrite::Applied(true) => Ok(active_manifest(manifest)),
        crate::catalog::ControllerWrite::Applied(false) => {
            anyhow::bail!(CATALOG_REPAIR_SUPERSEDED)
        }
        crate::catalog::ControllerWrite::Fenced => anyhow::bail!(CATALOG_REPAIR_FENCED),
    }
}

#[cfg(test)]
pub(crate) fn report_repair_outcome(
    probe: &mut Option<ReconcileScanProbe>,
    outcome: CatalogRepairProbeOutcome,
) {
    if let Some(probe) = probe.as_mut() {
        probe.repair_completed(outcome);
    }
}

#[cfg(test)]
pub(crate) fn repair_outcome(
    repaired: &Result<crate::catalog::ControllerWrite<bool>>,
) -> CatalogRepairProbeOutcome {
    match repaired {
        Ok(crate::catalog::ControllerWrite::Applied(true)) => CatalogRepairProbeOutcome::Applied,
        Ok(crate::catalog::ControllerWrite::Applied(false)) => {
            CatalogRepairProbeOutcome::Superseded
        }
        Ok(crate::catalog::ControllerWrite::Fenced) => CatalogRepairProbeOutcome::Fenced,
        Err(_) => CatalogRepairProbeOutcome::Failed,
    }
}
