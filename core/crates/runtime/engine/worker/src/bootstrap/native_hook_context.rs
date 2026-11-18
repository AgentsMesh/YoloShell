pub(crate) struct NativeHookContext {
    pub(crate) executable: PathBuf,
    pub(crate) home: PathBuf,
    pub(crate) codex_socket: PathBuf,
}

impl NativeHookContext {
    pub(crate) fn new(
        namespace: &WorkerNamespace,
        paths: &Paths,
        terminal_id: Uuid,
        executable: PathBuf,
    ) -> Result<Self> {
        crate::paths::validation::validate_absolute(&executable)?;
        Ok(Self {
            executable,
            home: paths.home().to_path_buf(),
            codex_socket: namespace.codex_hook_socket(terminal_id)?,
        })
    }

    pub(crate) fn configure(&self, command: &mut CommandBuilder) {
        command.env(
            crate::codex_hooks::NATIVE_HOOK_EXECUTABLE_ENV,
            &self.executable,
        );
        command.env(crate::codex_hooks::NATIVE_HOOK_HOME_ENV, &self.home);
        command.env(crate::codex_hooks::SOCKET_ENV, &self.codex_socket);
    }
}
