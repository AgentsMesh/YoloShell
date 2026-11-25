use super::*;

impl Paths {
    pub fn new(home: impl Into<PathBuf>) -> Self {
        let home = super::platform::platform_path(&home.into());
        Self {
            home,
            socket_root_override: std::env::var_os(constants::SOCKET_ROOT_ENVIRONMENT)
                .map(PathBuf::from)
                .map(|root| super::platform::platform_path(&root)),
        }
    }

    pub fn home(&self) -> &Path {
        &self.home
    }

    pub fn pty_custody(&self, id: uuid::Uuid) -> PathBuf {
        self.session_dir(id).join(constants::PTY_CUSTODY_FILE_NAME)
    }

    pub fn reboot_migration_intent(&self, id: uuid::Uuid) -> PathBuf {
        self.session_dir(id)
            .join(constants::REBOOT_MIGRATION_INTENT_FILE_NAME)
    }

    pub fn reboot_preparation_authority(&self, id: uuid::Uuid) -> PathBuf {
        self.session_dir(id)
            .join(constants::REBOOT_PREPARATION_AUTHORITY_FILE_NAME)
    }

    pub fn try_acquire_session_lifecycle_lock(
        &self,
        terminal_id: uuid::Uuid,
    ) -> Result<Option<private_file_lock::PrivateFileLock>> {
        private_file_lock::PrivateFileLock::try_acquire(
            &self
                .session_dir(terminal_id)
                .join(constants::SESSION_LIFECYCLE_LOCK_FILE_NAME),
            constants::SESSION_LIFECYCLE_LOCK_MAX_BYTES,
        )
    }

    #[doc(hidden)]
    pub fn session_lifecycle_lock_path(&self, terminal_id: uuid::Uuid) -> PathBuf {
        self.session_dir(terminal_id)
            .join(constants::SESSION_LIFECYCLE_LOCK_FILE_NAME)
    }

    pub(crate) fn socket_root_override(&self) -> Option<&Path> {
        self.socket_root_override.as_deref()
    }

    pub fn new_with_socket_root(home: impl Into<PathBuf>, root: PathBuf) -> Self {
        Self {
            home: super::platform::platform_path(&home.into()),
            socket_root_override: Some(super::platform::platform_path(&root)),
        }
    }
}
