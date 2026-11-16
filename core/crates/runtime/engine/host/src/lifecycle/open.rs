impl Host {
    pub fn open(config: HostConfig) -> Result<Self> {
        validate_home(&config.home)?;
        let paths = Paths::new(&config.home);
        let workspace_home = paths.home().to_path_buf();
        let lock_path = crate::coordination::lock_path(
            crate::coordination::CoordinationScope::HostHome,
            paths.home(),
            None,
        )?;
        let runtime_executable = config.runtime_executable.clone().map(Ok).unwrap_or_else(std::env::current_exe)?;
        let home_lock = InstanceLock::acquire(&lock_path, HOST_HOME_LOCK_LABEL)?;
        let home = crate::paths::directory::PrivateDirectory::open_or_create_strict(paths.home())?;
        home.ensure_descendant(&paths.runtime_dir())?;
        let sessions = home.ensure_descendant(&paths.sessions_dir())?;
        let socket_namespace = Arc::new(paths.retain_socket_namespace()?);
        let default_socket = socket_namespace.host_socket()?;
        let socket = config.socket.clone().unwrap_or(default_socket.clone());
        validate_socket_namespace_selection(
            &socket,
            &default_socket,
            socket_namespace.root_path(),
        )?;
        ensure_socket_is_not_lock(&socket, &lock_path)?;
        validate_socket_path(&socket)?;
        let endpoint_lock_path = socket_lock_path(&socket)?;
        let endpoint_lock = InstanceLock::acquire(&endpoint_lock_path, HOST_SOCKET_LOCK_LABEL)?;
        let socket_parent = socket.parent().context("host socket path has no parent")?;
        let socket_parent = crate::paths::directory::PrivateDirectory::ensure(socket_parent)?;
        let worker_sockets = socket_namespace.directory()?;
        let namespace = crate::namespace_lease::NamespaceLease::from_directories(vec![
            (HOST_HOME_NAMESPACE_LABEL, home),
            (HOST_SESSIONS_NAMESPACE_LABEL, sessions),
            (HOST_SOCKET_NAMESPACE_LABEL, socket_parent),
            (HOST_WORKER_SOCKET_NAMESPACE_LABEL, worker_sockets),
        ])?;
        crate::durable_reboot::settle_all_pending(&paths, &namespace)?;
        let durable_generation =
            catalog_rebuild::durable_controller_generation(&paths, &namespace)?;
        let catalog_entry = namespace.file(&paths.catalog())?;
        let (mut catalog, _) = Catalog::open_rebuildable_retained(catalog_entry, || {
            catalog_rebuild::durable_catalog_manifests(&paths, &namespace)
        })?;
        namespace.verify()?;
        catalog.integrity_check()?;
        let creation_ledger_path = paths.home().join(CREATION_LEDGER_FILE_NAME);
        let creation_receipts =
            CreationReceiptStore::open_retained(namespace.directory(paths.home())?)?;
        let mut creation_capabilities =
            CreationLedger::open_entry(namespace.file(&creation_ledger_path)?)?;
        namespace.verify()?;
        creation_capabilities.recover_completed(&creation_ledger_path, &creation_receipts)?;
        namespace.verify()?;
        let secondary_custody_holder = crate::pty_custody::CustodyHolderIdentity::capture()?;
        let controller_generation = catalog.claim_controller_generation(durable_generation)?;
        #[cfg(test)]
        pause_after_controller_claim(paths.home(), controller_generation);
        namespace.verify()?;
        let agent_registry = Arc::new(crate::agent::agent_registry_from_runtime(&config.provider_runtime));
        let provider_process_scanner = Arc::new(crate::agent::IsolatedProcessScanner::new(
            runtime_executable.clone(),
            agent_registry.process_environment_keys().into_iter().map(str::to_owned).collect(),
        )?);
        Ok(Self {
            config,
            paths,
            socket,
            runtime_executable,
            catalog: Arc::new(Mutex::new(catalog)),
            creation_capabilities: Mutex::new(creation_capabilities),
            creation_receipts,
            attachment_clients: Arc::new(tokio::sync::Semaphore::new(MAX_HOST_ATTACHMENTS)),
            session_locks: SessionLocks::default(),
            reconcile_gate: Arc::new(AsyncMutex::new(())),
            request_drain: request_drain::HostRequestDrain::default(),
            parked_resume_retries: Mutex::new(HashMap::new()),
            pending_recovery_writes: Mutex::new(HashMap::new()),
            secondary_custody: secondary_custody::SecondaryCustodyRegistry::default(),
            secondary_custody_holder,
            #[cfg(test)]
            catalog_test_probes: CatalogTestProbes::default(),
            #[cfg(test)]
            automatic_resume_test_probes: AutomaticResumeTestProbes::default(),
            watchdog_schedule: WatchdogSchedule::default(),
            worker_status_timeout: WORKER_STATUS_TIMEOUT,
            worker_ready_timeout: WORKER_READY_TIMEOUT,
            worker_ready_poll_interval: WORKER_READY_POLL_INTERVAL,
            resume_provider_binding_timeout: RESUME_PROVIDER_BINDING_TIMEOUT,
            resume_provider_binding_poll_interval: RESUME_PROVIDER_BINDING_POLL_INTERVAL,
            provider_process_scanner,
            agent_registry,
            provider_process_scans: ProviderProcessScanCoordinator::new(),
            boot_id: process::boot_id(),
            controller_generation,
            namespace: Arc::new(namespace),
            socket_namespace,
            instance_ownership: HostInstanceOwnership::new([home_lock, endpoint_lock]),
            worker_connections: worker_connections::WorkerConnections::default(),
            watch: agent_watch::HostWatch::new(),
            workspace: yolo_host_workspace::WorkspaceService::from_environment(&workspace_home),
        })
    }

    pub fn socket_path(&self) -> &Path {
        &self.socket
    }

    pub fn controller_generation(&self) -> u64 {
        self.controller_generation
    }
}

#[cfg(test)]
include!("open_generation_probe.rs");
include!("server.rs");
include!("ingress.rs");
