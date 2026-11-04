struct OwnerCancellationControl {
    _directory: tempfile::TempDir,
    context: OwnerCancellationContext,
    settled: bool,
}

impl OwnerCancellationControl {
    fn create() -> Result<Self> {
        let directory = tempfile::tempdir()?;
        let context = OwnerCancellationContext {
            directory: directory.path().to_path_buf(),
            nonce: Uuid::new_v4().to_string(),
        };
        Ok(Self {
            _directory: directory,
            context,
            settled: false,
        })
    }

    fn configure(&self, command: &mut std::process::Command) {
        command.env(
            OWNER_CANCELLATION_CONTROL_DIRECTORY_ENV,
            &self.context.directory,
        );
        command.env(OWNER_CANCELLATION_CONTROL_NONCE_ENV, &self.context.nonce);
    }

    fn verify(&self) -> Result<()> {
        self.context
            .require_stage(OwnerCancellationStage::Started)?;
        self.context
            .require_stage(OwnerCancellationStage::Completed)?;
        self.context.require_identity_inventory()
    }

    fn settle(&mut self) -> Result<()> {
        let result = settle_owner_cancellation_helpers(&self.context);
        self.settled = result.is_ok();
        result
    }
}

impl Drop for OwnerCancellationControl {
    fn drop(&mut self) {
        if !self.settled {
            let _ = settle_owner_cancellation_helpers(&self.context);
        }
    }
}

async fn wait_for_owner_cancellation_request(fixture: &Fixture) -> TerminationTombstone {
    let path = fixture
        .state
        .paths
        .terminate_tombstone(fixture.terminal_id());
    let tombstone: TerminationTombstone =
        tokio::time::timeout(OWNER_CANCELLATION_REQUEST_TIMEOUT, async {
            loop {
                if fixture.state.termination_reason() == Some(ExitReason::UserTerminate)
                    && let Ok(tombstone) = read_json(&path)
                {
                    return tombstone;
                }
                tokio::time::sleep(PROCESS_DISCOVERY_POLL_INTERVAL).await;
            }
        })
        .await
        .expect(OWNER_CANCELLATION_REQUEST_DETAIL);
    assert_eq!(
        tombstone.validate_for(fixture.terminal_id(), fixture.state.runtime_epoch()),
        Ok(())
    );
    tombstone
}

async fn prepare_owner_cancellation_leaf(marker: &Path, fixture: &Fixture) {
    release_root_first_trigger(marker);
    let intermediate_ready = marker.with_extension(ROOT_FIRST_INTERMEDIATE_READY_EXTENSION);
    wait_for_marker_file(
        &intermediate_ready,
        marker,
        &fixture.state,
        OWNER_CANCELLATION_SETUP_TIMEOUT,
    )
    .await;
    let intermediate_release = marker.with_extension(ROOT_FIRST_INTERMEDIATE_RELEASE_EXTENSION);
    std::fs::write(intermediate_release, EMPTY_MARKER).unwrap();
    let reopen_ready = marker.with_extension(ROOT_FIRST_REOPEN_READY_EXTENSION);
    wait_for_marker_file(
        &reopen_ready,
        marker,
        &fixture.state,
        OWNER_CANCELLATION_SETUP_TIMEOUT,
    )
    .await;
    let reopen_release = marker.with_extension(ROOT_FIRST_REOPEN_RELEASE_EXTENSION);
    std::fs::write(reopen_release, EMPTY_MARKER).unwrap();
}
