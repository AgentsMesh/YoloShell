use super::*;

impl SocketNamespaceLocation {
    pub(super) fn resolve(home: &Path, root_override: Option<&Path>) -> Result<Self> {
        let location = Self::locate(home, root_override)?;
        admission::validate_existing(home, &location)?;
        Ok(location)
    }

    pub(super) fn locate(home: &Path, root_override: Option<&Path>) -> Result<Self> {
        validate_home(home)?;
        let canonical = yolo_coordination_core::canonical_logical_path(home)?;
        let effective_user_id = unsafe { libc::geteuid() };
        let digest = hash::socket_digest(&canonical, effective_user_id);
        let location = Self::from_selection(
            effective_user_id,
            digest,
            root_selection::SocketRootSelection::new(effective_user_id, root_override)?,
        );
        location.validate_generated_paths()?;
        Ok(location)
    }

    pub(super) fn from_selection(
        effective_user_id: u32,
        digest: [u8; SOCKET_FULL_HASH_BYTES],
        selection: root_selection::SocketRootSelection,
    ) -> Self {
        let root = selection.path;
        let directory = root.join(hash::short_socket_hash(&digest));
        Self {
            root,
            directory,
            binding: NamespaceBinding {
                schema_version: SOCKET_BINDING_SCHEMA_VERSION,
                effective_user_id,
                home_digest: hash::full_socket_hash(&digest),
            },
            overridden_root: selection.overridden,
        }
    }

    pub(super) fn validate_generated_paths(&self) -> Result<()> {
        socket_stage::validate_parent(&self.directory)?;
        socket_path::validate(&self.directory.join(HOST_SOCKET_FILE_NAME))?;
        self.hub_socket()?;
        self.connector_socket()?;
        socket_path::validate(&self.directory.join(PUBLIC_CLIENT_SOCKET_FILE_NAME))?;
        socket_path::validate(
            &self
                .directory
                .join(format!("{}{SOCKET_FILE_EXTENSION}", Uuid::nil())),
        )?;
        endpoints::checked_claude_hook_path(&self.directory, Uuid::nil())?;
        endpoints::checked_codex_hook_path(&self.directory, Uuid::nil())?;
        self.custody_guardian_socket(Uuid::nil()).map(drop)
    }
}
