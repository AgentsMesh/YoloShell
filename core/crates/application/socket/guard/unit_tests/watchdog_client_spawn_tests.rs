use super::*;
use std::time::Instant;
use yolo_process::ProcessDiscoveryCapability;

#[path = "../test_support/watchdog_client_spawn_tests.rs"]
mod support;
use support::*;

#[test]
fn mismatched_discovery_value_closes_stdin_and_reaps_the_child() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(EOF_MARKER_NAME);
    let discovery = ProcessDiscoveryCapability::generate().unwrap();
    let frame = root_frame(&discovery, Mismatch::Value, directory.path());
    let (child, identity) = spawn_script(EOF_SCRIPT, &frame, &[path_text(&marker)]);

    let detail = rejected(child, discovery);

    assert!(marker.exists());
    assert!(detail.contains(WATCHDOG_HANDSHAKE_DETAIL), "{detail}");
    assert!(detail.contains(TEST_DIAGNOSTIC), "{detail}");
    assert_reaped(identity);
}

#[test]
fn mismatched_discovery_reader_forces_and_reaps_the_child() {
    let directory = tempfile::tempdir().unwrap();
    let discovery = ProcessDiscoveryCapability::generate().unwrap();
    let frame = root_frame(&discovery, Mismatch::Reader, directory.path());
    let (child, identity) = spawn_script(
        FORCE_REAP_SCRIPT,
        &frame,
        &[SLEEP_EXECUTABLE, SLEEP_DURATION_ARGUMENT],
    );
    let started = Instant::now();

    let detail = rejected(child, discovery);

    assert!(started.elapsed() >= WATCHDOG_DROP_TIMEOUT);
    assert!(started.elapsed() < SLEEP_DURATION);
    assert!(detail.contains(TEST_DIAGNOSTIC), "{detail}");
    assert_reaped(identity);
}

#[test]
fn mismatched_discovery_writer_joins_the_diagnostic_reader() {
    let directory = tempfile::tempdir().unwrap();
    let marker = directory.path().join(EOF_MARKER_NAME);
    let discovery = ProcessDiscoveryCapability::generate().unwrap();
    let frame = root_frame(&discovery, Mismatch::Writer, directory.path());
    let (child, identity) = spawn_script(EOF_SCRIPT, &frame, &[path_text(&marker)]);

    let detail = rejected(child, discovery);

    assert!(marker.exists());
    assert!(detail.contains(TEST_DIAGNOSTIC), "{detail}");
    assert_reaped(identity);
}

#[test]
fn mismatched_handshake_cancels_a_descendant_inherited_stderr_reader() {
    let directory = tempfile::tempdir().unwrap();
    let holder_path = directory.path().join(STDERR_HOLDER_PID_NAME);
    let discovery = ProcessDiscoveryCapability::generate().unwrap();
    let frame = root_frame(&discovery, Mismatch::Value, directory.path());
    let (child, identity) = spawn_script(
        INHERITED_STDERR_SCRIPT,
        &frame,
        &[
            SLEEP_EXECUTABLE,
            STDERR_HOLDER_DURATION_ARGUMENT,
            path_text(&holder_path),
        ],
    );
    let started = Instant::now();

    let detail = rejected(child, discovery);
    let holder = StderrHolder::capture(&holder_path);

    assert!(started.elapsed() < STDERR_HOLDER_DURATION);
    assert!(detail.contains(TEST_DIAGNOSTIC), "{detail}");
    assert_reaped(identity);
    holder.settle();
}
