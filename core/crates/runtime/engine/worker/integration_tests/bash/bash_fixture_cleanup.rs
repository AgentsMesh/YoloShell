use super::*;

#[path = "../../test_support/integration/bash/bash_fixture_cleanup.rs"]
mod support;
use self::support::constants::*;

#[test]
fn ordinary_bash_fixture_closes_aliases_and_reaps_after_setup_failures() -> Result<()> {
    for role in [READER_ERROR_ROLE, UNWIND_ROLE] {
        let directory = tempfile::tempdir()?;
        let evidence = directory.path().join(EVIDENCE_FILE);
        let mut command = ConcurrentRuntimeErrorProcess::test_command(HELPER_TEST)?;
        command.env(ROLE_ENV, role).env(EVIDENCE_ENV, &evidence);
        let mut helper = ConcurrentRuntimeErrorProcess::spawn_command(command)?;
        helper.deadline = Instant::now() + RUNTIME_ERROR_CUSTODY_TIMEOUT;
        helper.finish()?;
        let identity = wait_for_runtime_error_watchdog_descendant(&evidence)?;
        assert_eq!(
            identity.observe(),
            process::ProcessIdentityObservation::Gone
        );
    }
    Ok(())
}

#[test]
fn bash_fixture_cleanup_helper() {
    let Ok(role) = std::env::var(ROLE_ENV) else {
        return;
    };
    let evidence = PathBuf::from(std::env::var_os(EVIDENCE_ENV).unwrap());
    let fixture = support::fixture();
    let state = Arc::clone(&fixture.state);
    let identity = state.pty_root_identity();
    let master = support::master_descriptor(&fixture);
    atomic_write_json(
        &evidence,
        &RuntimeErrorWatchdogProcessRecord {
            pid: identity.pid,
            start_time: identity.start_time,
        },
    )
    .unwrap();

    match role.as_str() {
        READER_ERROR_ROLE => {
            let failure = support::reject_reader_construction(fixture).unwrap_err();
            assert_eq!(failure.raw_os_error(), Some(libc::EMFILE));
        }
        UNWIND_ROLE => {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _owned_fixture = fixture;
                std::panic::panic_any(INJECTED_UNWIND);
            }));
            assert_eq!(
                result.unwrap_err().downcast_ref::<&str>(),
                Some(&INJECTED_UNWIND)
            );
        }
        _ => unreachable!(),
    }
    support::assert_reclaimed(&state, master);
}
