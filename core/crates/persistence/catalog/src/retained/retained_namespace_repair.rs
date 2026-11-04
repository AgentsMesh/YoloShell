impl RetainedCatalogNamespace {
    pub(super) fn register_repair(
        main: PrivateFileEntry,
        logical_path: PathBuf,
        expected: FileIdentity,
    ) -> Result<Self> {
        let alias_shard = Arc::new(super::catalog_alias_lease::CatalogAliasShard::retain(
            &main,
        )?);
        let entries = super::retained_registry_identity::candidate_entries(&main)?
            .into_iter()
            .enumerate()
            .map(|(index, entry)| {
                repair_artifact(entry, Arc::clone(&alias_shard), &logical_path, index)
            })
            .collect::<Result<Vec<_>>>()?;
        bind_repair_main(&entries, expected)?;
        let inner = Arc::new(NamespaceInner {
            alias_shard,
            path: logical_path,
            entries,
        });
        Ok(Self(super::retained_registry::register(inner)?))
    }

    pub(super) fn require_main_object(&self, expected: FileIdentity) -> Result<()> {
        let current = repair_main(&self.0.entries)?
            .entry()
            .inspect(MAX_CATALOG_ARTIFACT_BYTES)?
            .context("retained repair main identity is unavailable")?;
        anyhow::ensure!(
            current.same_object(expected),
            "retained repair main artifact was replaced"
        );
        Ok(())
    }
}

fn bind_repair_main(entries: &[RetainedArtifact], expected: FileIdentity) -> Result<()> {
    let main = repair_main(entries)?;
    let current = main
        .entry()
        .inspect(MAX_CATALOG_ARTIFACT_BYTES)?
        .context("retained repair main artifact is unavailable")?;
    anyhow::ensure!(
        current.same_object(expected),
        "retained repair main artifact was replaced before registration"
    );
    let file = super::secure_vfs_retained_file::reopen(main.entry())?;
    main.bind(super::secure_vfs_identity::FileIdentity::inspect(
        &file.metadata()?,
        main.path(),
    )?)
}

fn repair_main(entries: &[RetainedArtifact]) -> Result<&RetainedArtifact> {
    entries
        .first()
        .context("retained repair namespace has no main artifact")
}

fn repair_artifact(
    entry: PrivateFileEntry,
    alias_shard: Arc<super::catalog_alias_lease::CatalogAliasShard>,
    logical_path: &Path,
    index: usize,
) -> Result<RetainedArtifact> {
    let path = repair_artifact_path(logical_path, index)?;
    Ok(RetainedArtifact::new_at_path(entry, alias_shard, path))
}

fn repair_artifact_path(logical_path: &Path, index: usize) -> Result<PathBuf> {
    if index == MAIN_ARTIFACT_INDEX {
        return Ok(logical_path.to_path_buf());
    }
    let suffix = match index {
        JOURNAL_ARTIFACT_INDEX => SQLITE_JOURNAL_SUFFIX,
        WAL_ARTIFACT_INDEX => SQLITE_WAL_SUFFIX,
        SHM_ARTIFACT_INDEX => SQLITE_SHM_SUFFIX,
        _ => anyhow::bail!(RETAINED_REPAIR_ARTIFACT_INDEX_ERROR),
    };
    let mut name = logical_path
        .file_name()
        .context("retained repair artifact has no file name")?
        .to_owned();
    name.push(suffix);
    Ok(logical_path.with_file_name(name))
}
