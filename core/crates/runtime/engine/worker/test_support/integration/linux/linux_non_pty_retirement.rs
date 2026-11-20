const LINUX_FIXTURE_FROZEN_FILE: &str = "cgroup.freeze";
const LINUX_FIXTURE_FROZEN_VALUE: &str = "1";
const LINUX_RETAINED_FIXTURE_TIMEOUT: &str = "non-PTY fixture did not enter retained cleanup";

fn finish_linux_retained_fixture(
    owner: &GatedOwner,
    fixture: &Fixture,
    retained: &process::LiveProcessIdentity,
) {
    let deadline = Instant::now() + FORCE_SETTLEMENT_INTEGRATION_TIMEOUT;
    loop {
        let latest = owner.reaper_state.latest();
        if let Some(snapshot) = latest.as_ref()
            && snapshot.root_waited
            && snapshot.custody.as_ref().is_some_and(|custody| {
                custody.kind == pty_process_custody::PtyProcessCustodyKind::GoneReported
            })
        {
            assert!(!snapshot.completion_published);
            assert!(!snapshot.observer_only);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "{LINUX_RETAINED_FIXTURE_TIMEOUT}: {latest:#?}"
        );
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
    assert!(retained.try_matches().unwrap());
    assert_ne!(fixture.state.status().status, TerminalRuntimeStatus::Exited);
    assert!(
        !fixture
            .state
            .paths
            .exit_record(fixture.terminal_id())
            .exists()
    );
    let record: process::linux_containment::LinuxContainmentRecord =
        read_json(&process::linux_containment::record_path(
            &fixture.state.paths.session_dir(fixture.terminal_id()),
            fixture.state.runtime_epoch(),
        ))
        .unwrap();
    let freeze = record
        .delegated_root
        .join(record.leaf_name)
        .join(LINUX_FIXTURE_FROZEN_FILE);
    assert_eq!(
        std::fs::read_to_string(freeze).unwrap().trim(),
        LINUX_FIXTURE_FROZEN_VALUE
    );
    // This external fixture owner ends its own retained child only after proving deferral.
    assert!(retained.terminate_retained_fixture().unwrap());
}
