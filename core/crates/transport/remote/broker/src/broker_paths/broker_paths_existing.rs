use super::*;

impl BrokerPaths {
    /// Capture an existing private directory without creating it or changing its mode.
    pub fn open_existing(state_dir: impl Into<PathBuf>) -> Result<Self> {
        let state_dir = state_dir.into();
        security::validate_absolute(&state_dir, BROKER_STATE_DIRECTORY_LABEL)?;
        let state = PrivateDirectory::open(&state_dir)?;
        let anchor = BrokerStateAnchor::capture(&state)?;
        let instance_lock = crate::coordination::existing_lock_path(
            CoordinationScope::BrokerInstance,
            &state_dir,
            None,
        )?;
        let launch_publication_lock = crate::coordination::existing_lock_path(
            CoordinationScope::BrokerLaunchPublication,
            &state_dir,
            None,
        )?;
        let reconcile_lock = crate::coordination::existing_lock_path(
            CoordinationScope::BrokerReconcile,
            &state_dir,
            None,
        )?;
        let paths = Self {
            state_dir,
            instance_lock,
            launch_publication_lock,
            reconcile_lock,
            state: Arc::new(state),
            anchor,
        };
        security::validate_socket_path(&paths.runtime_socket())?;
        security::validate_socket_path(&paths.management_socket())?;
        paths.verify_state()?;
        Ok(paths)
    }
}
