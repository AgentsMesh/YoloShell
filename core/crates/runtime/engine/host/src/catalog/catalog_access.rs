pub(crate) const CATALOG_TASK_JOIN_CONTEXT: &str = "catalog blocking task failed";
pub(crate) const CATALOG_GENERATION_FENCED: &str =
    "catalog write was fenced by a newer host generation";
#[cfg(test)]
pub(crate) const CATALOG_UPSERT_TEST_FAILURE: &str = "injected catalog upsert failure";

#[cfg(test)]
#[derive(Default)]
pub(crate) struct CatalogTestProbes {
    pub(crate) scan: Mutex<Option<ReconcileScanProbe>>,
    pub(crate) operations: Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>,
    pub(crate) operation_completions: Mutex<Option<tokio::sync::mpsc::UnboundedSender<()>>>,
    pub(crate) recovery_publication: Mutex<Option<RecoveryPublicationProbe>>,
    pub(crate) restore_decision: Mutex<Option<ReconcileRestoreProbe>>,
    pub(crate) reconcile_panic: Mutex<Option<Uuid>>,
    pub(crate) tombstone_consume: Mutex<Option<Box<dyn FnOnce() -> Result<()> + Send>>>,
    pub(crate) creation_commit_owned: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    pub(crate) lifecycle_revalidation: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    pub(crate) archive_publication: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    pub(crate) manifest_publication_commit: Mutex<Option<Box<dyn FnOnce() -> Result<()> + Send>>>,
    pub(crate) upsert_failure: Mutex<bool>,
}

impl Host {
    pub(crate) async fn catalog_operation<T, F>(&self, operation: F) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Catalog) -> Result<T> + Send + 'static,
    {
        let catalog = Arc::clone(&self.catalog);
        #[cfg(test)]
        let probe = lock_recover(&self.catalog_test_probes.operations).clone();
        #[cfg(test)]
        let completion = lock_recover(&self.catalog_test_probes.operation_completions).clone();
        tokio::task::spawn_blocking(move || {
            #[cfg(test)]
            if let Some(probe) = probe {
                let _ = probe.send(());
            }
            let result = operation(&mut lock_recover(&catalog));
            #[cfg(test)]
            if let Some(completion) = completion {
                let _ = completion.send(());
            }
            result
        })
        .await
        .context(CATALOG_TASK_JOIN_CONTEXT)?
    }

    pub(crate) async fn catalog_operation_before<T, F>(
        &self,
        deadline: HostOperationDeadline,
        operation: F,
    ) -> Result<T>
    where
        T: Send + 'static,
        F: FnOnce(&mut Catalog, CreationCatalogCommitGate) -> Result<T> + Send + 'static,
    {
        let gate = CreationCatalogCommitGate::new(deadline);
        let worker_gate = gate.clone();
        let operation = self.catalog_operation(move |catalog| operation(catalog, worker_gate));
        if let Some(result) = deadline.timeout(operation).await {
            return result;
        }
        match gate.cancel() {
            CreationCatalogTimeoutDisposition::Cancelled => {
                Err(anyhow::Error::new(CreationPublicationDeadlineExpired))
            }
            CreationCatalogTimeoutDisposition::CommitOwned => {
                Err(anyhow::Error::new(CreationPublicationCommitInFlight))
            }
        }
    }

    pub(crate) async fn catalog_get(&self, terminal_id: Uuid) -> Result<Option<TerminalManifest>> {
        self.catalog_operation(move |catalog| catalog.get(terminal_id))
            .await
    }

    pub(crate) async fn catalog_list_tolerant(
        &self,
        include_archived: bool,
    ) -> Result<crate::catalog::CatalogListing> {
        self.catalog_operation(move |catalog| catalog.list_tolerant(include_archived))
            .await
    }

    pub(crate) async fn catalog_list_complete(
        &self,
        include_archived: bool,
    ) -> Result<Vec<TerminalManifest>> {
        self.catalog_operation(move |catalog| catalog.list(include_archived))
            .await
    }

    pub(crate) async fn catalog_insert(&self, manifest: &TerminalManifest) -> Result<()> {
        let manifest = manifest.clone();
        let generation = self.controller_generation;
        self.catalog_operation(move |catalog| {
            require_catalog_write(catalog.insert_for_controller(generation, &manifest)?)
        })
        .await
    }

    pub(crate) async fn catalog_insert_before(
        &self,
        manifest: &TerminalManifest,
        deadline: HostOperationDeadline,
    ) -> Result<()> {
        let manifest = manifest.clone();
        let generation = self.controller_generation;
        #[cfg(test)]
        let commit_owned = lock_recover(&self.catalog_test_probes.creation_commit_owned).take();
        self.catalog_operation_before(deadline, move |catalog, commit| {
            require_catalog_write(catalog.insert_for_controller_with_commit_fence(
                generation,
                &manifest,
                || {
                    commit.claim()?;
                    #[cfg(test)]
                    if let Some(probe) = commit_owned {
                        probe();
                    }
                    Ok(())
                },
            )?)
        })
        .await
    }

    pub(crate) async fn catalog_upsert(&self, manifest: &TerminalManifest) -> Result<()> {
        #[cfg(test)]
        if std::mem::take(&mut *lock_recover(&self.catalog_test_probes.upsert_failure)) {
            bail!(CATALOG_UPSERT_TEST_FAILURE);
        }
        let manifest = manifest.clone();
        let generation = self.controller_generation;
        self.catalog_operation(move |catalog| {
            require_catalog_write(catalog.upsert_for_controller(generation, &manifest)?)
        })
        .await
    }

    pub(crate) async fn catalog_upsert_before(
        &self,
        manifest: &TerminalManifest,
        deadline: HostOperationDeadline,
    ) -> Result<()> {
        let manifest = manifest.clone();
        let generation = self.controller_generation;
        #[cfg(test)]
        let commit_owned = lock_recover(&self.catalog_test_probes.creation_commit_owned).take();
        self.catalog_operation_before(deadline, move |catalog, commit| {
            require_catalog_write(catalog.upsert_for_controller_with_commit_fence(
                generation,
                &manifest,
                || {
                    commit.claim()?;
                    #[cfg(test)]
                    if let Some(probe) = commit_owned {
                        probe();
                    }
                    Ok(())
                },
            )?)
        })
        .await
    }
}

pub(crate) fn require_catalog_write<T>(write: crate::catalog::ControllerWrite<T>) -> Result<T> {
    match write {
        crate::catalog::ControllerWrite::Applied(result) => Ok(result),
        crate::catalog::ControllerWrite::Fenced => bail!(CATALOG_GENERATION_FENCED),
    }
}

include!("catalog_manifest_publication.rs");
