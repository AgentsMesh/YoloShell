use super::*;

impl Catalog {
    pub fn open_rebuildable<F>(path: &Path, load_manifests: F) -> Result<(Self, bool)>
    where
        F: FnOnce() -> Result<Vec<TerminalManifest>>,
    {
        Self::open_rebuildable_with(path, None, load_manifests)
    }

    pub fn open_rebuildable_retained<F>(
        entry: crate::paths::private_file::PrivateFileEntry,
        load_manifests: F,
    ) -> Result<(Self, bool)>
    where
        F: FnOnce() -> Result<Vec<TerminalManifest>>,
    {
        let namespace = retained_namespace::RetainedCatalogNamespace::register(entry)?;
        let path = namespace.path().to_path_buf();
        Self::open_rebuildable_with(&path, Some(namespace), load_manifests)
    }

    fn open_rebuildable_with<F>(
        path: &Path,
        retained: Option<retained_namespace::RetainedCatalogNamespace>,
        load_manifests: F,
    ) -> Result<(Self, bool)>
    where
        F: FnOnce() -> Result<Vec<TerminalManifest>>,
    {
        let manifests =
            load_manifests().context("load durable manifests for catalog validation")?;
        let (open_error, generation_floor) = match open_checked(path, retained.clone()) {
            Ok(catalog) => match super::validation::validate_durable_snapshot(&catalog, &manifests)
            {
                Ok(()) => return Ok((catalog, false)),
                Err(error) => {
                    let generation = catalog.controller_generation()?;
                    drop(catalog);
                    (error, Some(generation))
                }
            },
            Err(error) => (error, None),
        };
        if !super::rebuild_reason::is_rebuildable(&open_error) {
            return Err(open_error);
        }
        match &retained {
            Some(namespace) => namespace.quarantine(),
            None => super::artifacts::quarantine_catalog(path),
        }
        .context("quarantine invalid catalog index")?;
        let mut catalog =
            Self::open_with_namespace(path, retained).context("create rebuilt catalog index")?;
        if let Some(generation) = generation_floor {
            catalog.restore_controller_generation_floor(generation)?;
        }
        for manifest in &manifests {
            catalog
                .upsert(manifest)
                .context("populate rebuilt catalog index")?;
        }
        catalog
            .integrity_check()
            .context("validate rebuilt catalog index")?;
        super::validation::validate_durable_snapshot(&catalog, &manifests)
            .context("validate rebuilt catalog contents")?;
        tracing::warn!(%open_error, catalog = %path.display(), "rebuilt invalid catalog index");
        Ok((catalog, true))
    }
}

fn open_checked(
    path: &Path,
    retained: Option<retained_namespace::RetainedCatalogNamespace>,
) -> Result<Catalog> {
    let catalog = Catalog::open_with_namespace(path, retained)?;
    catalog.integrity_check()?;
    Ok(catalog)
}
