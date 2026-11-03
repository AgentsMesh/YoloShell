pub(crate) const DURABLE_MANIFEST_INVALID: &str = "durable manifest identity or schema mismatch";

pub(crate) enum CatalogScanItem {
    Manifest(Option<TerminalManifest>),
    Repair(CatalogRepairRequest),
}

pub(crate) fn reconciliation_manifests_blocking(
    paths: &Paths,
    namespace: Arc<crate::namespace_lease::NamespaceLease>,
    catalog: Arc<Mutex<Catalog>>,
    generation: u64,
    #[cfg(test)] probe: &mut Option<ReconcileScanProbe>,
) -> (Vec<TerminalManifest>, usize) {
    let listing = match lock_recover(&catalog).list_tolerant(true) {
        Ok(listing) => listing,
        Err(error) => {
            tracing::error!(%error, "catalog cannot be scanned by the recovery watchdog");
            return (Vec::new(), 1);
        }
    };
    let catalog_weak = Arc::downgrade(&catalog);
    let namespace_weak = Arc::downgrade(&namespace);
    let catalog_entry = namespace.file(&paths.catalog()).ok();
    let mut diagnostic_count = listing.diagnostics.len();
    let mut manifests = Vec::new();
    let mut repairs = Vec::new();
    for catalog_manifest in listing.manifests {
        let terminal_id = catalog_manifest.terminal_id;
        match reconcile_catalog_row(
            paths,
            &namespace,
            catalog_entry.as_ref(),
            catalog_weak.clone(),
            namespace_weak.clone(),
            catalog_manifest,
        ) {
            Ok(CatalogScanItem::Manifest(Some(manifest))) => manifests.push(manifest),
            Ok(CatalogScanItem::Manifest(None)) => {}
            Ok(CatalogScanItem::Repair(request)) => repairs.push(request),
            Err(error) => {
                diagnostic_count += 1;
                tracing::error!(%terminal_id, %error, "reject stale catalog session");
            }
        }
    }
    for diagnostic in listing.diagnostics {
        tracing::error!(
            catalog_row = %diagnostic.row_key,
            terminal_id = ?diagnostic.terminal_id,
            detail = %diagnostic.detail,
            "invalid catalog session retained for recovery"
        );
        let Some(terminal_id) = diagnostic.terminal_id else {
            continue;
        };
        let Some(expectation) = diagnostic.expectation else {
            continue;
        };
        match repair_catalog_from_durable(
            paths,
            &namespace,
            catalog_entry.as_ref(),
            catalog_weak.clone(),
            namespace_weak.clone(),
            terminal_id,
            expectation,
        ) {
            Ok(request) => repairs.push(request),
            Err(error) => tracing::error!(%terminal_id, %error, "repair invalid catalog session"),
        }
    }
    // A cancelled spawn_blocking task may outlive Host, so repairs retain only physical entries.
    drop(catalog_entry);
    drop(catalog);
    drop(namespace);
    for request in repairs {
        let terminal_id = request.manifest.terminal_id;
        let counts_diagnostic_on_failure = request.source.counts_diagnostic_on_failure();
        match repair_catalog_request(
            paths,
            request,
            generation,
            #[cfg(test)]
            probe,
        ) {
            Ok(Some(manifest)) => manifests.push(manifest),
            Ok(None) => {}
            Err(error) => {
                if counts_diagnostic_on_failure {
                    diagnostic_count += 1;
                }
                tracing::error!(%terminal_id, %error, "catalog repair failed");
            }
        }
    }
    (manifests, diagnostic_count)
}

pub(crate) fn reconcile_catalog_row(
    paths: &Paths,
    namespace: &crate::namespace_lease::NamespaceLease,
    catalog_entry: Option<&crate::paths::private_file::PrivateFileEntry>,
    catalog: Weak<Mutex<Catalog>>,
    namespace_weak: Weak<crate::namespace_lease::NamespaceLease>,
    indexed: TerminalManifest,
) -> Result<CatalogScanItem> {
    let durable = durable_repair_manifest(paths, namespace, indexed.terminal_id)?;
    if serde_json::to_vec(&indexed)? == serde_json::to_vec(&durable)? {
        return Ok(CatalogScanItem::Manifest(active_manifest(durable)));
    }
    let expectation = crate::catalog::CatalogRowExpectation::Valid(indexed);
    Ok(CatalogScanItem::Repair(repair_request(
        catalog_entry,
        catalog,
        namespace_weak,
        expectation,
        durable,
        CatalogRepairSource::Indexed,
    )?))
}

pub(crate) fn repair_catalog_from_durable(
    paths: &Paths,
    namespace: &crate::namespace_lease::NamespaceLease,
    catalog_entry: Option<&crate::paths::private_file::PrivateFileEntry>,
    catalog: Weak<Mutex<Catalog>>,
    namespace_weak: Weak<crate::namespace_lease::NamespaceLease>,
    terminal_id: Uuid,
    expectation: crate::catalog::CatalogRowExpectation,
) -> Result<CatalogRepairRequest> {
    let durable = durable_repair_manifest(paths, namespace, terminal_id)?;
    repair_request(
        catalog_entry,
        catalog,
        namespace_weak,
        expectation,
        durable,
        CatalogRepairSource::Diagnostic,
    )
}

pub(crate) fn active_manifest(manifest: TerminalManifest) -> Option<TerminalManifest> {
    (!manifest.archived).then_some(manifest)
}

pub(crate) fn durable_repair_manifest(
    paths: &Paths,
    namespace: &crate::namespace_lease::NamespaceLease,
    terminal_id: Uuid,
) -> Result<TerminalManifest> {
    let manifest = namespace.read_json::<TerminalManifest>(&paths.manifest(terminal_id))?;
    anyhow::ensure!(
        manifest.terminal_id == terminal_id
            && manifest.schema_version == SESSION_MANIFEST_SCHEMA_VERSION,
        DURABLE_MANIFEST_INVALID
    );
    Ok(manifest)
}
