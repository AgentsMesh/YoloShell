const ROOT_FIRST_POST_LAUNCH_PHASE_COUNT: u32 = 6;
const ROOT_FIRST_HELPER_WATCHDOG_MARGIN: Duration = FORCE_SETTLEMENT_INTEGRATION_TIMEOUT;
const ROOT_FIRST_HELPER_WATCHDOG_TIMEOUT: Duration = ROOT_FIRST_EXIT_TIMEOUT
    .saturating_mul(ROOT_FIRST_POST_LAUNCH_PHASE_COUNT)
    .saturating_add(FORCE_SETTLEMENT_INTEGRATION_TIMEOUT)
    .saturating_add(ROOT_FIRST_HELPER_WATCHDOG_MARGIN);
const ROOT_FIRST_HELPER_WATCHDOG_EXTENSION: &str = "helper-watchdog-expired";
const ROOT_FIRST_HELPER_WATCHDOG_DETAIL: &str =
    "root-first leaf reached its fallback watchdog instead of production cleanup";
const ROOT_FIRST_MARKER_PARENT_REMOVED_DETAIL: &str =
    "root-first marker directory disappeared before production cleanup completed";
#[cfg(target_os = "macos")]
const ROOT_FIRST_PATH_HOLDER_DETAIL: &str =
    "root-first descendant was absent from the PTY path-holder query";
#[cfg(target_os = "macos")]
const ROOT_FIRST_PTY_PROOF_TIMEOUT_DETAIL: &str = "root-first descendant PTY proof stayed deferred";

#[cfg(target_os = "macos")]
fn assert_root_first_leaf_pty_proof(
    capability: &FixturePtyCapability,
    fixture: &Fixture,
    descendant: process::ProcessIdentity,
) {
    let proof = capability.proof(fixture);
    let deadline = Instant::now() + ROOT_FIRST_EXIT_TIMEOUT;
    loop {
        match proof.matches_process_for_test(descendant.pid).unwrap() {
            Some(matches) => {
                assert!(matches, "{ROOT_FIRST_PTY_PROOF_DETAIL}");
                break;
            }
            None => {
                assert!(
                    Instant::now() < deadline,
                    "{ROOT_FIRST_PTY_PROOF_TIMEOUT_DETAIL}"
                );
                std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
            }
        }
    }
    assert!(
        proof
            .path_holders_for_test()
            .unwrap()
            .contains(&descendant.pid),
        "{ROOT_FIRST_PATH_HOLDER_DETAIL}"
    );
}

fn wait_for_root_first_leaf_lifetime(marker: &Path) {
    let deadline = Instant::now() + ROOT_FIRST_HELPER_WATCHDOG_TIMEOUT;
    let parent = marker.parent().expect(ROOT_FIRST_MARKER_PARENT_DETAIL);
    while parent.exists() {
        if Instant::now() >= deadline {
            std::fs::write(
                marker.with_extension(ROOT_FIRST_HELPER_WATCHDOG_EXTENSION),
                EMPTY_MARKER,
            )
            .unwrap();
            return;
        }
        std::thread::sleep(PROCESS_DISCOVERY_POLL_INTERVAL);
    }
}

fn assert_root_first_leaf_cleanup_won(marker: &Path) {
    assert!(
        marker
            .parent()
            .expect(ROOT_FIRST_MARKER_PARENT_DETAIL)
            .exists(),
        "{ROOT_FIRST_MARKER_PARENT_REMOVED_DETAIL}"
    );
    assert!(
        !marker
            .with_extension(ROOT_FIRST_HELPER_WATCHDOG_EXTENSION)
            .exists(),
        "{ROOT_FIRST_HELPER_WATCHDOG_DETAIL}"
    );
}
