use crate::namespace_lease::{NamespaceOwnershipLoss, NamespaceVerification};

pub(crate) const SESSION_REMOVAL_RETRY_DETAIL: &str =
    "session removal check is indeterminate; retrying without termination";
pub(crate) const SESSION_REMOVAL_TERMINATE_DETAIL: &str = "terminating orphaned session worker";
pub(crate) const SESSION_REMOVAL_TERMINATE_FAILURE_DETAIL: &str =
    "failed to terminate orphaned PTY process tree";
pub(crate) const SESSION_REMOVAL_MANIFEST_MISMATCH: &str =
    "removed session manifest does not belong to the settling worker";
pub(crate) const SESSION_REMOVAL_MANIFEST_RESTORED: &str =
    "removed session manifest was restored before worker settlement";
pub(crate) const SESSION_REMOVAL_OBSERVER_JOIN_DETAIL: &str = "join session removal observation";

pub(crate) enum AgentProjectionShutdown {
    Invalidate,
    ManifestRevoked(RemovedTerminalManifest),
}

pub(crate) struct RemovedTerminalManifest {
    pub(crate) path: PathBuf,
}

pub(crate) enum SessionRemoval {
    NamespaceLost {
        path: PathBuf,
        _proof: NamespaceOwnershipLoss,
    },
    ManifestMissing(PathBuf),
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum SessionRemovalSettlement {
    RetryObservation,
    Terminated,
}

impl SessionRemoval {
    pub(crate) fn path(&self) -> &Path {
        match self {
            Self::NamespaceLost { path, .. } | Self::ManifestMissing(path) => path,
        }
    }

    pub(crate) fn removed_manifest(&self) -> Option<RemovedTerminalManifest> {
        match self {
            Self::ManifestMissing(path) => Some(RemovedTerminalManifest { path: path.clone() }),
            Self::NamespaceLost { .. } => None,
        }
    }

    pub(crate) async fn terminate(
        self,
        state: &Arc<WorkerState>,
        terminal_id: Uuid,
    ) -> Result<SessionRemovalSettlement> {
        let permit = state.admit_system_mutation().await;
        match self.remains_removed(state) {
            Ok(true) => {}
            Ok(false) => return Ok(SessionRemovalSettlement::RetryObservation),
            Err(error) => {
                tracing::warn!(%error, SESSION_REMOVAL_RETRY_DETAIL);
                return Ok(SessionRemovalSettlement::RetryObservation);
            }
        }
        tracing::warn!(
            %terminal_id,
            path = %self.path().display(),
            reason = ORPHAN_REASON,
            SESSION_REMOVAL_TERMINATE_DETAIL
        );
        state.begin_system_termination(&permit, ExitReason::Lost);
        if let Err(error) = terminate_pty_processes(state, permit, true).await {
            tracing::warn!(%error, SESSION_REMOVAL_TERMINATE_FAILURE_DETAIL);
        }
        wait_for_reader_shutdown(Arc::clone(state)).await?;
        Ok(SessionRemovalSettlement::Terminated)
    }

    pub(crate) fn remains_removed(&self, state: &WorkerState) -> Result<bool> {
        match self {
            Self::NamespaceLost { .. } => Ok(true),
            Self::ManifestMissing(path) => {
                anyhow::ensure!(
                    *path == state.paths.manifest(state.terminal_id()),
                    SESSION_REMOVAL_MANIFEST_MISMATCH
                );
                state.namespace.file_is_missing(path)
            }
        }
    }
}

impl RemovedTerminalManifest {
    pub(crate) fn verify_missing(&self, state: &WorkerState) -> Result<()> {
        let expected = state.paths.manifest(state.terminal_id());
        anyhow::ensure!(self.path == expected, SESSION_REMOVAL_MANIFEST_MISMATCH);
        anyhow::ensure!(
            state.namespace.file_is_missing(&self.path)?,
            SESSION_REMOVAL_MANIFEST_RESTORED
        );
        Ok(())
    }
}

pub(crate) enum SessionRemovalObservation {
    Present,
    Removed(SessionRemoval),
    Indeterminate(anyhow::Error),
}

pub(crate) async fn wait_for_session_removal(state: Arc<WorkerState>) -> SessionRemoval {
    wait_for_session_removal_with_observer(move || observe_session_removal(&state)).await
}

pub(crate) async fn wait_for_session_removal_with_observer(
    observe: impl Fn() -> SessionRemovalObservation + Send + Sync + 'static,
) -> SessionRemoval {
    let observe = Arc::new(observe);
    loop {
        let scan = Arc::clone(&observe);
        let observed = tokio::task::spawn_blocking(move || scan())
            .await
            .context(SESSION_REMOVAL_OBSERVER_JOIN_DETAIL)
            .and_then(into_session_removal);
        match observed {
            Ok(Some(removal)) => return removal,
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(%error, SESSION_REMOVAL_RETRY_DETAIL);
            }
        }
        // Keep one read-only scan in flight and never catch up missed polling cycles.
        tokio::time::sleep(ORPHAN_CHECK_INTERVAL).await;
    }
}

pub(crate) fn observe_session_removal(state: &WorkerState) -> SessionRemovalObservation {
    let home = state.paths.home().to_path_buf();
    match classify_namespace_observation(home.clone(), state.namespace.verification()) {
        SessionRemovalObservation::Present => {}
        settled => return settled,
    }
    let manifest = state.paths.manifest(state.terminal_id());
    match state.namespace.file_is_missing(&manifest) {
        Ok(false) => SessionRemovalObservation::Present,
        Ok(true) => SessionRemovalObservation::Removed(SessionRemoval::ManifestMissing(manifest)),
        Err(error) => match classify_namespace_observation(home, state.namespace.verification()) {
            SessionRemovalObservation::Removed(removal) => {
                SessionRemovalObservation::Removed(removal)
            }
            SessionRemovalObservation::Present | SessionRemovalObservation::Indeterminate(_) => {
                SessionRemovalObservation::Indeterminate(error)
            }
        },
    }
}

pub(crate) fn classify_namespace_observation(
    home: PathBuf,
    verification: NamespaceVerification,
) -> SessionRemovalObservation {
    match verification {
        NamespaceVerification::Verified => SessionRemovalObservation::Present,
        NamespaceVerification::IdentityLost { proof, .. } => {
            SessionRemovalObservation::Removed(SessionRemoval::NamespaceLost {
                path: home,
                _proof: proof,
            })
        }
        NamespaceVerification::Indeterminate(error) => {
            SessionRemovalObservation::Indeterminate(error)
        }
    }
}

pub(crate) fn into_session_removal(
    observation: SessionRemovalObservation,
) -> Result<Option<SessionRemoval>> {
    match observation {
        SessionRemovalObservation::Present => Ok(None),
        SessionRemovalObservation::Removed(removal) => Ok(Some(removal)),
        SessionRemovalObservation::Indeterminate(error) => Err(error),
    }
}
