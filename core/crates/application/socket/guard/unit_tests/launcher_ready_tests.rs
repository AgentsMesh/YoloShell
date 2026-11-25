use super::*;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::time::Duration;

const DELAYED_READY_BUDGET: Duration = Duration::from_millis(20);
const INVALID_MARKER: &[u8] = b"X";

#[test]
fn an_exec_child_held_before_user_space_ready_cannot_cross_the_tracking_gate() {
    let (mut parent, mut child) = Input::pair().unwrap();
    let (release, held) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        held.recv().unwrap();
        child.write_all(LAUNCHER_READY_MARKER).unwrap();
        let mut start = [0; LAUNCHER_READY_MARKER.len()];
        child.read_exact(&mut start).unwrap();
        assert_eq!(start, *LAUNCHER_READY_MARKER);
    });
    // The child owns its control endpoint but has not published readiness.
    // A bounded attempt must fail, independent of scheduler speed or /proc timing.
    assert!(wait_for_ready(&mut parent, DELAYED_READY_BUDGET).is_err());
    release.send(()).unwrap();
    wait(&mut parent).unwrap();
    parent.write_all(LAUNCHER_READY_MARKER).unwrap();
    writer.join().unwrap();
}

#[test]
fn missing_or_wrong_ready_frames_fail_without_releasing_the_start_gate() {
    for bytes in [b"".as_slice(), INVALID_MARKER] {
        let (mut parent, mut child) = Input::pair().unwrap();
        child.write_all(bytes).unwrap();
        drop(child);
        assert!(wait(&mut parent).is_err());
    }
}
