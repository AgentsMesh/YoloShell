const OWNER_CANCELLATION_HELPER_SETTLE_DETAIL: &str =
    "published cancellation helpers did not settle";
const OWNER_CANCELLATION_TEST_PID_BASE: u32 = 1_000_000;
const OWNER_CANCELLATION_TEST_START_TIME: u64 = 1;
const OWNER_CANCELLATION_UNKNOWN_RETRY_PASS_COUNT: usize = 2;
const OWNER_CANCELLATION_UNKNOWN_SIGNAL_DETAIL: &str =
    "unknown helper identity must not be signaled";

fn settle_owner_cancellation_helpers(context: &OwnerCancellationContext) -> Result<()> {
    settle_owner_cancellation_helpers_until(
        context,
        Instant::now() + OWNER_CANCELLATION_HELPER_SETTLE_TIMEOUT,
        process::ProcessIdentity::observe,
        |identity| identity.signal(libc::SIGKILL),
    )
}

fn assert_owner_cancellation_identity_gone(identity: process::ProcessIdentity) {
    assert_eq!(
        identity.observe(),
        process::ProcessIdentityObservation::Gone
    );
}

fn settle_cancelled_owner_cleanup(
    fixture: &Fixture,
    owner: &mut GatedOwner,
    capability: &mut FixturePtyCapability,
) {
    owner.wait_for_root_exit(OWNER_CANCELLATION_ROOT_EXIT_TIMEOUT);
    capability.assert_synthetic_writer_present(fixture);
    let manager_inspection = capability.inspection_proof(fixture);
    let cleanup_path = owner.release_after_cleanup_started(OWNER_CANCELLATION_SETTLEMENT_TIMEOUT);
    #[cfg(target_os = "macos")]
    assert_eq!(
        cleanup_path,
        RootExitCleanupPath::FaultReaperDeferred,
        "{FAULT_REAP_DEFERRED_PATH_DETAIL}"
    );
    owner.assert_retained_runtime_status(&fixture.state, cleanup_path);
    capability.assert_queued_writer_present();
    capability.release_writer_after_root_exit(&owner.root_live);
    capability.assert_manager_excluded(manager_inspection, OWNER_CANCELLATION_SETTLEMENT_TIMEOUT);
    owner.wait_for_settlement(OWNER_CANCELLATION_SETTLEMENT_TIMEOUT);
    owner.wait_for_cleanup_completion(cleanup_path, OWNER_CANCELLATION_SETTLEMENT_TIMEOUT);
    assert_fault_reap_runtime_completed(&fixture.state, cleanup_path);
    owner.join();
}

fn settle_owner_cancellation_helpers_until<O, S>(
    context: &OwnerCancellationContext,
    deadline: Instant,
    mut observe: O,
    mut signal: S,
) -> Result<()>
where
    O: FnMut(process::ProcessIdentity) -> process::ProcessIdentityObservation,
    S: FnMut(process::ProcessIdentity) -> Result<bool>,
{
    let mut failure = None;
    loop {
        let mut unsettled = false;
        for &role in OWNER_CANCELLATION_HELPER_ROLES {
            match context.read_identity(role) {
                Ok(Some(identity)) => match observe(identity) {
                    process::ProcessIdentityObservation::Matching => {
                        unsettled = true;
                        if let Err(error) = signal(identity) {
                            failure.get_or_insert(error);
                        }
                    }
                    process::ProcessIdentityObservation::Gone => {}
                    process::ProcessIdentityObservation::Unknown => unsettled = true,
                },
                Ok(None) => {}
                Err(error) => {
                    unsettled = true;
                    failure.get_or_insert(error);
                }
            }
        }
        if !unsettled {
            return failure.map_or(Ok(()), Err);
        }
        if Instant::now() >= deadline {
            let timeout = anyhow::anyhow!(OWNER_CANCELLATION_HELPER_SETTLE_DETAIL);
            return Err(failure.unwrap_or(timeout));
        }
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

#[test]
fn owner_cancellation_unknown_identity_observation_retries_until_gone() {
    let control = owner_cancellation_test_control();
    let observations = std::cell::Cell::new(usize::default());
    let first_pass = OWNER_CANCELLATION_HELPER_ROLES.len();
    settle_owner_cancellation_helpers_until(
        &control.context,
        Instant::now() + OWNER_CANCELLATION_HELPER_SETTLE_TIMEOUT,
        |_| {
            let current = observations.get();
            observations.set(current.saturating_add(1));
            if current < first_pass {
                process::ProcessIdentityObservation::Unknown
            } else {
                process::ProcessIdentityObservation::Gone
            }
        },
        |_| panic!("{OWNER_CANCELLATION_UNKNOWN_SIGNAL_DETAIL}"),
    )
    .unwrap();
    assert_eq!(
        observations.get(),
        first_pass * OWNER_CANCELLATION_UNKNOWN_RETRY_PASS_COUNT
    );
}

#[test]
fn owner_cancellation_unknown_identity_observation_fails_closed_at_deadline() {
    let control = owner_cancellation_test_control();
    let observations = std::cell::Cell::new(usize::default());
    let error = settle_owner_cancellation_helpers_until(
        &control.context,
        Instant::now(),
        |_| {
            observations.set(observations.get().saturating_add(1));
            process::ProcessIdentityObservation::Unknown
        },
        |_| panic!("{OWNER_CANCELLATION_UNKNOWN_SIGNAL_DETAIL}"),
    )
    .unwrap_err();
    assert_eq!(observations.get(), OWNER_CANCELLATION_HELPER_ROLES.len());
    assert!(
        error
            .to_string()
            .contains(OWNER_CANCELLATION_HELPER_SETTLE_DETAIL)
    );
}

fn owner_cancellation_test_control() -> OwnerCancellationControl {
    let control = OwnerCancellationControl::create().unwrap();
    for (offset, &role) in OWNER_CANCELLATION_HELPER_ROLES.iter().enumerate() {
        control
            .context
            .publish(
                role,
                process::ProcessIdentity {
                    pid: OWNER_CANCELLATION_TEST_PID_BASE + u32::try_from(offset).unwrap(),
                    start_time: OWNER_CANCELLATION_TEST_START_TIME,
                },
            )
            .unwrap();
    }
    control
}
