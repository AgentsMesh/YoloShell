#[derive(Clone)]
pub(super) struct CodexHookBindRequest {
    socket_directory: Arc<crate::paths::directory::PrivateDirectory>,
    path: PathBuf,
    paths: crate::paths::Paths,
    registry: Arc<super::authority_registry::CodexAuthorityRegistry>,
    terminal_id: Uuid,
    runtime_epoch: Uuid,
    shell: crate::process::ProcessIdentity,
    policy: super::hook_policy::CodexHookPolicy,
    process_source: Arc<dyn crate::agent::ProcessSource>,
    native_namespace: Arc<crate::namespace_lease::NamespaceLease>,
}

impl CodexHookBindRequest {
    pub(super) fn capture_native(
        namespace: &WorkerNamespace,
        paths: &crate::paths::Paths,
        registry: Arc<super::authority_registry::CodexAuthorityRegistry>,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
        shell: crate::process::ProcessIdentity,
        hook_executable: &Path,
        process_source: Arc<dyn crate::agent::ProcessSource>,
    ) -> Result<Self> {
        let path = namespace.codex_hook_socket(terminal_id)?;
        observe_durable_codex_authority_floor(namespace, paths, terminal_id, &registry)?;
        let directory = socket_directory(namespace, &path)?;
        let policy = super::hook_policy::CodexHookPolicy::new(
            terminal_id,
            runtime_epoch,
            hook_executable.to_path_buf(),
        )?;
        Ok(Self {
            socket_directory: Arc::new(directory),
            path,
            paths: paths.clone(),
            registry,
            terminal_id,
            runtime_epoch,
            shell,
            policy,
            process_source,
            native_namespace: retain_native_namespace(namespace, paths, terminal_id)?,
        })
    }

    pub(super) async fn bind(
        &self,
        generation: super::cancellation::IngressCancellation,
    ) -> Result<CodexHookServer> {
        let (retained_directory, publication_directory) =
            retain_socket_directory(self.socket_directory.try_clone()?)?;
        let router = Arc::new(super::router::CodexSessionRouter::with_publication_clock(
            self.paths.clone(),
            self.terminal_id,
            self.runtime_epoch,
            self.registry.route_publication_clock(),
        )?);
        let settlements = super::blocking_custody::BlockingSettlementCustody::default();
        let transaction = Arc::new(super::hook_context::CodexHookContext::new(
            self.paths.clone(),
            Arc::clone(&self.registry),
            Arc::clone(&router),
            self.policy.clone(),
            self.shell,
            Arc::clone(&self.process_source),
            settlements.clone(),
            generation.clone(),
            Arc::clone(&self.native_namespace),
        )?);
        let mut recovered_round = None;
        let published = {
            let recovery = super::recovery::CodexRecoveryContext::new(
                &self.paths,
                Arc::clone(&self.registry),
                router.as_ref(),
                self.shell,
                Arc::clone(&self.process_source),
            );
            publish_after_recovery(publication_directory, &self.path, || {
                recovered_round = Some(recovery.recover()?);
                #[cfg(test)]
                super::recovery_test_hook::run_before_socket_publication();
                Ok(())
            })
            .await
        };
        let (listener, publication) = match published {
            Ok(published) => published,
            Err(error) => {
                return Err(match recovered_round {
                    Some(round) => round.reject(error),
                    None => error,
                });
            }
        };
        let Some(round) = recovered_round else {
            let error = anyhow::anyhow!(CODEX_RECOVERY_ROUND_ERROR);
            return Err(reject_published(listener, publication, None, error));
        };
        #[cfg(test)]
        super::recovery_test_hook::run_after_socket_publication();
        if let Err(error) = round
            .verify()
            .and_then(|()| verify_socket_directory(&retained_directory))
        {
            return Err(reject_published(listener, publication, Some(round), error));
        }
        round.accept();
        Ok(CodexHookServer::finish_bind(
            retained_directory,
            self.path.clone(),
            listener,
            publication,
            router,
            transaction,
            settlements,
            generation,
        ))
    }
}

fn observe_durable_codex_authority_floor(
    namespace: &WorkerNamespace,
    paths: &crate::paths::Paths,
    terminal_id: Uuid,
    registry: &super::authority_registry::CodexAuthorityRegistry,
) -> Result<()> {
    let authority_store =
        crate::agent::codex::authority::CodexBindingAuthorityStore::from_binding_path(
            &paths.agent_binding(terminal_id),
        )?;
    if let Some(authority) = authority_store.load_in_namespace(namespace.lease())? {
        registry.observe_route_publication_floor(authority.version.route_revision);
    }
    Ok(())
}

fn reject_published(
    listener: UnixListener,
    publication: crate::paths::socket_activation::SocketPublication,
    round: Option<super::recovery_round::CodexRecoveryRound>,
    cause: anyhow::Error,
) -> anyhow::Error {
    drop(listener);
    let withdrawal = publication.withdraw();
    let rollback = round.map(|round| round.rollback()).unwrap_or(Ok(()));
    let error = match rollback {
        Ok(()) => cause,
        Err(cleanup) => anyhow::Error::new(cleanup).context(cause),
    };
    match withdrawal {
        Ok(()) => error,
        Err(cleanup) => cleanup.context(error),
    }
}
