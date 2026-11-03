#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_TEST: &str =
    "worker::tests::real_pty_exit::fixture_pty_capability_writer_holder";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_ROLE_ENV: &str = "YOLO_FIXTURE_CAPABILITY_HOLDER_ROLE";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_ROLE: &str = "exact-writer-holder";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_DESCRIPTOR_ENV: &str = "YOLO_FIXTURE_CAPABILITY_HOLDER_FD";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_READY_ENV: &str = "YOLO_FIXTURE_CAPABILITY_HOLDER_READY";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_RELEASE_ENV: &str = "YOLO_FIXTURE_CAPABILITY_HOLDER_RELEASE";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_PARENT_PID_ENV: &str = "YOLO_FIXTURE_CAPABILITY_HOLDER_PARENT_PID";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_PARENT_START_ENV: &str =
    "YOLO_FIXTURE_CAPABILITY_HOLDER_PARENT_START";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_READY_FILE: &str = "writer.ready";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_RELEASE_FILE: &str = "writer.release";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_ENV_DETAIL: &str =
    "fixture capability holder environment is invalid";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_IDENTITY_DETAIL: &str =
    "fixture capability holder identity is unavailable";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_READY_DETAIL: &str =
    "fixture capability holder did not become ready";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_EARLY_EXIT_DETAIL: &str =
    "fixture capability holder exited before release";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_EXIT_DETAIL: &str =
    "fixture capability holder did not settle through release or cleanup";
pub(super) struct FixturePtyCapability {
    value: String,
    #[cfg(target_os = "macos")]
    authority: Arc<process::MacosProcessCapabilityManagerReader>,
    #[cfg(target_os = "macos")]
    holder: Option<FixturePtyCapabilityHolder>,
    #[cfg(target_os = "macos")]
    queued: Option<FixtureQueuedPtyCapability>,
}

#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_HOLDER_MISSING_DETAIL: &str =
    "fixture process capability holder is missing";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_MANAGER_CAPTURED_DETAIL: &str =
    "fixture PTY proof captured its manager process";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_MANAGER_SCAN_DETAIL: &str =
    "fixture PTY proof could not inspect its manager process";
#[cfg(target_os = "macos")]
const FIXTURE_CAPABILITY_SYNTHETIC_WRITER_MISSING_DETAIL: &str =
    "fixture PTY proof did not observe its synthetic writer";
impl FixturePtyCapability {
    fn synthetic_descriptor() -> Self {
        #[cfg(target_os = "macos")]
        let (authority, writer) = process::MacosProcessCapabilityIssuer::create()
            .unwrap()
            .transfer_for_test()
            .unwrap();
        #[cfg(target_os = "macos")]
        let holder = FixturePtyCapabilityHolder::launch(writer, &authority).unwrap();
        Self {
            value: Uuid::new_v4().to_string(),
            #[cfg(target_os = "macos")]
            authority,
            #[cfg(target_os = "macos")]
            holder: Some(holder),
            #[cfg(target_os = "macos")]
            queued: None,
        }
    }

    pub(super) fn value(&self) -> &str {
        &self.value
    }

    fn proof(&self, fixture: &Fixture) -> process::PtySessionProof {
        let master = lock_recover(&fixture.state.master);
        let proof = process::PtySessionProof::capture_bound_for_test(
            master.as_ref().unwrap().as_ref(),
            self.value.clone(),
        )
        .unwrap();
        self.install_authority(proof)
    }

    pub(super) fn inspection_proof(&self, fixture: &Fixture) -> process::PtySessionProof {
        let tty_name = lock_recover(&fixture.state.master)
            .as_ref()
            .unwrap()
            .tty_name()
            .unwrap();
        let proof = process::PtySessionProof::capture(tty_name, self.value.clone()).unwrap();
        self.install_authority(proof)
    }

    fn install_authority(&self, mut proof: process::PtySessionProof) -> process::PtySessionProof {
        #[cfg(target_os = "macos")]
        proof
            .install_process_capability_for_test(Arc::clone(&self.authority))
            .unwrap();
        proof
    }

    pub(super) fn assert_manager_excluded(
        &self,
        proof: process::PtySessionProof,
        timeout: Duration,
    ) {
        #[cfg(target_os = "macos")]
        {
            let deadline = Instant::now() + timeout;
            loop {
                match proof.matches_exact_writer_for_test(std::process::id()) {
                    Ok(false) => break,
                    Ok(true) => panic!("{FIXTURE_CAPABILITY_MANAGER_CAPTURED_DETAIL}"),
                    Err(error) => assert!(
                        Instant::now() < deadline,
                        "{FIXTURE_CAPABILITY_MANAGER_SCAN_DETAIL}: {error:#}"
                    ),
                }
                std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
            }
        }
        #[cfg(not(target_os = "macos"))]
        let _ = (proof, timeout);
    }

    pub(super) fn assert_synthetic_writer_present(&self, fixture: &Fixture) {
        #[cfg(target_os = "macos")]
        if let Some(queued) = self.queued.as_ref() {
            queued.assert_hidden_and_present(&self.authority);
            return;
        }
        #[cfg(target_os = "macos")]
        assert!(
            self.inspection_proof(fixture)
                .matches_exact_writer_for_test(
                    self.holder
                        .as_ref()
                        .expect(FIXTURE_CAPABILITY_HOLDER_MISSING_DETAIL)
                        .identity()
                        .pid,
                )
                .unwrap(),
            "{FIXTURE_CAPABILITY_SYNTHETIC_WRITER_MISSING_DETAIL}"
        );
        #[cfg(not(target_os = "macos"))]
        let _ = fixture;
    }

    pub(super) fn release_writer_after_root_exit(&mut self, root: &process::LiveProcessIdentity) {
        #[cfg(target_os = "macos")]
        {
            self.authority.confirm_root_exit_for_test(root).unwrap();
            if let Some(mut queued) = self.queued.take() {
                queued.release();
                return;
            }
            let mut holder = self
                .holder
                .take()
                .expect(FIXTURE_CAPABILITY_HOLDER_MISSING_DETAIL);
            holder.release_and_reap().unwrap();
        }
        #[cfg(not(target_os = "macos"))]
        let _ = root;
    }
}
