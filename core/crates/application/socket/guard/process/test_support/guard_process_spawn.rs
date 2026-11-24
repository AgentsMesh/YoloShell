use super::*;
use std::os::unix::process::CommandExt;

impl GuardProcess {
    pub(crate) fn spawn(leaf_state: LeafState, failure: FailureAction) -> Self {
        let mut process = Self::launch(leaf_state, failure);
        process.wait_for_ready();
        process.refresh_evidence();
        assert_eq!(process.leaf.is_some(), leaf_state != LeafState::None);
        process.watchdog = Some(process_support::capture_watchdog(
            process.guard_identity,
            process.leaf,
        ));
        if leaf_state == LeafState::Stopped {
            stop_leaf(process.leaf.unwrap());
        }
        process
    }

    pub(crate) fn spawn_unready(leaf_state: LeafState) -> Self {
        Self::launch(leaf_state, FailureAction::None)
    }

    pub(crate) fn wait_for_setup_evidence(&self) -> (ProcessIdentity, ProcessIdentity, PathBuf) {
        let deadline = Instant::now() + START_TIMEOUT;
        while !self.ready.exists() {
            assert!(Instant::now() < deadline, "{FIXTURE_TIMEOUT_DETAIL}");
            std::thread::sleep(POLL_INTERVAL);
        }
        let leaf = process_support::capture_identity(&self.leaf_identity_file);
        let watchdog = process_support::capture_watchdog(self.guard_identity, Some(leaf));
        let root = PathBuf::from(OsString::from_vec(
            std::fs::read(&self.socket_root_file).unwrap(),
        ));
        (watchdog, leaf, root)
    }

    fn launch(leaf_state: LeafState, failure: FailureAction) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let ready = directory.path().join(READY_FILE);
        let release = directory.path().join(RELEASE_FILE);
        let leaf_identity = directory.path().join(LEAF_IDENTITY_FILE);
        let late_leaf_identity = directory.path().join(LATE_LEAF_IDENTITY_FILE);
        let leaf_ready = directory.path().join(LEAF_READY_FILE);
        let socket_root_file = directory.path().join(SOCKET_ROOT_FILE);
        let failure_trigger = directory.path().join(FAILURE_TRIGGER_FILE);
        let force_request = directory.path().join(FORCE_SIGNAL_REQUEST_FILE);
        let force_acknowledgement = directory.path().join(FORCE_SIGNAL_ACKNOWLEDGEMENT_FILE);
        let refresh_request = directory.path().join(REFRESH_HANDOFF_REQUEST_FILE);
        let refresh_acknowledgement = directory.path().join(REFRESH_HANDOFF_ACKNOWLEDGEMENT_FILE);
        let arm_request = directory.path().join(DISCOVERY_ARM_REQUEST_FILE);
        let arm_acknowledgement = directory.path().join(DISCOVERY_ARM_ACKNOWLEDGEMENT_FILE);
        let holder_ready = directory.path().join(REFRESH_HOLDER_READY_FILE);
        let refresh_postcheck_witness = directory.path().join(REFRESH_POSTCHECK_WITNESS_FILE);
        let coordinator_retire_request = directory.path().join(COORDINATOR_RETIRE_REQUEST_FILE);
        let historical_witness_identity = directory.path().join(HISTORICAL_WITNESS_IDENTITY_FILE);
        let historical_witness_acknowledgement = directory
            .path()
            .join(HISTORICAL_WITNESS_ACKNOWLEDGEMENT_FILE);
        let refresh_evidence = (leaf_state == LeafState::ForkAfterRefresh).then(|| {
            RefreshEvidence::new(
                arm_request.clone(),
                arm_acknowledgement.clone(),
                holder_ready.clone(),
                refresh_request.clone(),
                coordinator_retire_request.clone(),
                refresh_acknowledgement.clone(),
                historical_witness_acknowledgement.clone(),
            )
        });
        let location = process_support::test_location();
        let mut command =
            process_support::guard_command(&location, leaf_state, failure, &ready, &release);
        command
            .env(FIXTURE_LEAF_IDENTITY_ENVIRONMENT, &leaf_identity)
            .env(FIXTURE_LEAF_READY_ENVIRONMENT, &leaf_ready)
            .env(FIXTURE_LATE_LEAF_IDENTITY_ENVIRONMENT, &late_leaf_identity)
            .env(FIXTURE_SOCKET_ROOT_ENVIRONMENT, &socket_root_file)
            .env(FIXTURE_FORCE_SIGNAL_REQUEST_ENVIRONMENT, &force_request)
            .env(
                FIXTURE_FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT,
                &force_acknowledgement,
            )
            .env(
                FIXTURE_REFRESH_HANDOFF_REQUEST_ENVIRONMENT,
                &refresh_request,
            )
            .env(
                FIXTURE_REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT,
                &refresh_acknowledgement,
            )
            .env(FIXTURE_DISCOVERY_ARM_REQUEST_ENVIRONMENT, &arm_request)
            .env(
                FIXTURE_DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT,
                &arm_acknowledgement,
            )
            .env(FIXTURE_REFRESH_HOLDER_READY_ENVIRONMENT, &holder_ready)
            .env(
                FIXTURE_COORDINATOR_RETIRE_REQUEST_ENVIRONMENT,
                &coordinator_retire_request,
            )
            .env(
                FIXTURE_HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT,
                &historical_witness_identity,
            )
            .env(
                FIXTURE_HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT,
                &historical_witness_acknowledgement,
            )
            .env(FAILURE_TRIGGER_ENVIRONMENT, &failure_trigger);
        if matches!(
            leaf_state,
            LeafState::ForkOnTerm | LeafState::ForkBeforeForce
        ) {
            command
                .env(FORCE_SIGNAL_REQUEST_ENVIRONMENT, &force_request)
                .env(
                    FORCE_SIGNAL_ACKNOWLEDGEMENT_ENVIRONMENT,
                    &force_acknowledgement,
                );
        }
        if leaf_state == LeafState::ForkAfterRefresh {
            command
                .env(
                    crate::fault_contract::HISTORICAL_WITNESS_IDENTITY_ENVIRONMENT,
                    &historical_witness_identity,
                )
                .env(
                    crate::fault_contract::HISTORICAL_WITNESS_ACKNOWLEDGEMENT_ENVIRONMENT,
                    &historical_witness_acknowledgement,
                )
                .env(DISCOVERY_ARM_REQUEST_ENVIRONMENT, &arm_request)
                .env(
                    DISCOVERY_ARM_ACKNOWLEDGEMENT_ENVIRONMENT,
                    &arm_acknowledgement,
                )
                .env(REFRESH_HANDOFF_REQUEST_ENVIRONMENT, &refresh_request)
                .env(
                    REFRESH_HANDOFF_ACKNOWLEDGEMENT_ENVIRONMENT,
                    &refresh_acknowledgement,
                )
                .env(
                    crate::fault_contract::REFRESH_POSTCHECK_WITNESS_ENVIRONMENT,
                    &refresh_postcheck_witness,
                );
        }
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == SYSTEM_CALL_FAILURE {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().expect(FIXTURE_SPAWN_DETAIL);
        let identity = process_support::capture_child_identity(&mut child);
        Self {
            _directory: directory,
            guard: Some(child),
            status: None,
            guard_identity: identity,
            watchdog: None,
            leaf: None,
            late_leaf: None,
            refresh_holder: None,
            socket_root: None,
            ready,
            release,
            failure_trigger,
            leaf_identity_file: leaf_identity,
            late_leaf_identity_file: late_leaf_identity,
            refresh_holder_identity_file: holder_ready,
            socket_root_file,
            refresh_postcheck_witness,
            refresh_evidence,
            leaf_state,
            failure,
            triggered: false,
            settled: false,
        }
    }
}
