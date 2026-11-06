use std::os::unix::fs::OpenOptionsExt;

impl Fixture {
    pub(super) fn new() -> Self {
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        Self::for_process(process)
    }

    pub(super) fn new_isolated() -> Self {
        let process = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        Self::for_process_in_scope(process, Uuid::new_v4(), Uuid::new_v4())
            .with_prepared_retirement_writer_lock()
    }

    pub(super) fn new_with_prepared_retirement_writer_lock() -> Self {
        Self::new().with_prepared_retirement_writer_lock()
    }

    pub(super) fn for_process(process: crate::process::ProcessIdentity) -> Self {
        Self::for_process_in_scope(process, Self::terminal_id(), Self::runtime_epoch())
    }

    fn for_process_in_scope(
        process: crate::process::ProcessIdentity,
        terminal_id: Uuid,
        runtime_epoch: Uuid,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let paths = crate::paths::Paths::new(directory.path().join(TEST_HOME_DIRECTORY));
        paths.ensure_session(terminal_id).unwrap();
        let shell = crate::process::ProcessIdentity::capture(std::process::id()).unwrap();
        let identity = HookIdentity {
            terminal_id: terminal_id,
            runtime_epoch,
            invocation_id: Uuid::new_v4(),
            hook_token: Uuid::from_u128(TOKEN_UUID),
            process: ProcessIdentity::new(process.pid, process.start_time),
            generation: Generation::INITIAL,
        };
        let source = super::process_support::stub_process_source(process);
        let (runtime, replay) = CodexHookRuntime::new_test(
            &paths,
            terminal_id,
            runtime_epoch,
            shell,
            identity.clone(),
            crate::codex_hooks::ACCEPTED_PROVIDER_VERSION,
            source,
        )
        .unwrap();
        Self {
            _directory: directory,
            paths,
            runtime,
            replay,
            identity,
            shell,
            process,
        }
    }

    fn with_prepared_retirement_writer_lock(self) -> Self {
        prepare_writer_lock(&self.paths, &self.identity);
        self
    }
}

pub(super) fn prepare_writer_lock(paths: &crate::paths::Paths, identity: &HookIdentity) {
    let artifacts = crate::codex_hooks::CodexHookArtifacts::new(
        paths,
        identity.terminal_id,
        identity.runtime_epoch,
        identity.invocation_id,
    )
    .unwrap();
    // First-create durability has dedicated paths tests; runtime tests start after provisioning.
    std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(crate::paths::constants::PRIVATE_FILE_MODE)
        .open(artifacts.retirement_writer_lock_path_for_test())
        .unwrap();
}
