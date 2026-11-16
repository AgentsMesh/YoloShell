use super::*;
use yolo_resilience_support::secondary_custody_adversarial_observation::ActiveCustodyObservation;

#[path = "guardian_fail_stop_support.rs"]
mod test_support;
use test_support::*;

const SESSION_NAME: &str = "guardian-fail-stop";
const BEFORE_HOLDER: &str = "guardian-before";
const AFTER_HOLDER: &str = "guardian-after";
const BEFORE_INPUT: &[u8] = b"guardian-before\n";
const AFTER_INPUT: &[u8] = b"guardian-after\n";
const BEFORE_OUTPUT: &[u8] = b"guardian-before";
const AFTER_OUTPUT: &[u8] = b"guardian-after";
const GUARDIAN_GONE_LOG_MARKER: &str = "detached PTY custody guardian is proven gone";

#[tokio::test(flavor = "current_thread")]
async fn exact_guardian_death_fail_stops_host_and_preserves_runtime_for_successor() {
    let home = tempfile::tempdir().unwrap();
    let host = spawn_host(home.path());
    let mut harness = Harness::start_with_host(home, host).await;
    let terminal_id = harness.create_cat(SESSION_NAME).await.manifest.terminal_id;
    let paths = harness.paths();
    let before: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    let host_a = harness.host_identity();
    let active_a = wait_for_observation(&harness, terminal_id, before.controller_generation).await;
    assert_activation(&active_a, &before, host_a);
    assert_terminal_round_trip(
        &harness,
        &before,
        BEFORE_HOLDER,
        BEFORE_INPUT,
        BEFORE_OUTPUT,
    )
    .await;

    let guardian_a = local_identity(active_a.guardian());
    assert!(guardian_a.signal(libc::SIGKILL).unwrap());
    wait_for_gone(guardian_a).await;
    discard_incomplete_coverage_profile(guardian_a.pid);
    let status = wait_for_host_fail_stop(&mut harness).await;
    assert!(!status.success());
    assert!(
        harness
            .host_stderr_tail()
            .contains(GUARDIAN_GONE_LOG_MARKER)
    );

    let stranded: RuntimeRecord = read_json(&paths.runtime_record(terminal_id)).unwrap();
    assert_same_runtime(&stranded, &before);
    assert_eq!(stranded.controller_generation, before.controller_generation);
    assert!(runtime_process(&stranded).matches());
    assert!(child_process(&stranded).matches());
    assert_eq!(host_a.observe(), process::ProcessIdentityObservation::Gone);

    harness.replace_host(spawn_host(harness.home.path())).await;
    let (after, active_b) = wait_for_successor(&harness, terminal_id, &before).await;
    let host_b = harness.host_identity();
    assert_activation(&active_b, &after, host_b);
    assert_same_runtime(&after, &before);
    assert!(after.controller_generation > before.controller_generation);
    assert_eq!(active_a.recovery_identity(), active_b.recovery_identity());
    let guardian_b = local_identity(active_b.guardian());
    assert_ne!(guardian_b, guardian_a);
    assert!(guardian_b.matches());
    assert_eq!(
        guardian_a.observe(),
        process::ProcessIdentityObservation::Gone
    );
    assert!(runtime_process(&after).matches());
    assert!(child_process(&after).matches());
    assert_terminal_takeover_round_trip(&harness, &after, AFTER_HOLDER, AFTER_INPUT, AFTER_OUTPUT)
        .await;
}
