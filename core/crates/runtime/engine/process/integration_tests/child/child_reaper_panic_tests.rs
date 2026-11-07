use super::*;
use crate::process::{DetachedStandardError, spawn_detached};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const OBSERVATION_INTERVAL: Duration = Duration::from_millis(5);
const WAIT_TIMEOUT: Duration = Duration::from_secs(1);
const WAITID_OPTIONS: libc::c_int = libc::WEXITED | libc::WNOHANG | libc::WNOWAIT;
const WAITPID_OPTIONS: libc::c_int = libc::WNOHANG;
const WAIT_FAILURE: libc::pid_t = -1;
const EXPECTED_CONTAINED_PANICS: usize = 2;

#[test]
fn worker_panic_retains_custody_and_enters_a_fail_closed_fallback() {
    let reaper = DetachedChildReaper::start_with(|shared| {
        std::thread::spawn(move || worker::run(shared));
        Ok(())
    })
    .unwrap();
    let first = reaper.reserve().unwrap();
    let issued_before_failure = reaper.reserve().unwrap();
    reaper.panic_after_next_receive();
    let first_pid = retain_sleeping_child(first);
    let waiter = reaper.clone();
    let wait_started = Instant::now();
    let wait = std::thread::spawn(move || {
        waiter.wait_for_termination_delivery(wait_started + WAIT_TIMEOUT)
    });

    while !lock_queue(&reaper.shared).worker_failed {
        std::thread::sleep(OBSERVATION_INTERVAL);
    }
    wait_for_contained_panics(&reaper, 1);
    let rejection = reaper.reserve().err().unwrap();
    assert!(rejection.to_string().contains(REAPER_WORKER_FAILED));
    reaper.panic_after_next_receive();
    let second_pid = retain_sleeping_child(issued_before_failure);
    wait_for_contained_panics(&reaper, EXPECTED_CONTAINED_PANICS);
    let wait_error = wait.join().unwrap().unwrap_err();
    assert!(wait_error.to_string().contains(REAPER_WORKER_FAILED));
    assert!(wait_started.elapsed() < WAIT_TIMEOUT);

    wait_until_reaped(first_pid);
    wait_until_reaped(second_pid);
    let queue = lock_queue(&reaper.shared);
    assert_eq!(queue.custody_count, usize::default());
    assert_eq!(queue.termination_count, usize::default());
}

fn wait_for_contained_panics(reaper: &DetachedChildReaper, expected: usize) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while reaper.shared.panic_payload_drops.load(Ordering::SeqCst) < expected {
        assert!(Instant::now() < deadline);
        std::thread::sleep(OBSERVATION_INTERVAL);
    }
}

impl DetachedChildReaper {
    fn panic_after_next_receive(&self) {
        self.shared
            .panic_after_receive
            .store(true, Ordering::SeqCst);
    }
}

fn retain_sleeping_child(reservation: DetachedChildReservation) -> u32 {
    let child = spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let process_id = child.id();
    reservation.retain(
        crate::process::DetachedChildCustody::capture(child).unwrap(),
        TerminationPolicy::Terminate,
    );
    process_id
}

fn wait_until_reaped(process_id: u32) {
    let deadline = Instant::now() + WAIT_TIMEOUT;
    while child_wait_authority_exists(process_id) {
        assert!(Instant::now() < deadline);
        std::thread::sleep(OBSERVATION_INTERVAL);
    }
    let waited = unsafe {
        libc::waitpid(
            process_id as libc::pid_t,
            std::ptr::null_mut(),
            WAITPID_OPTIONS,
        )
    };
    assert_eq!(waited, WAIT_FAILURE);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ECHILD)
    );
}

fn child_wait_authority_exists(process_id: u32) -> bool {
    let mut status = std::mem::MaybeUninit::<libc::siginfo_t>::zeroed();
    let result = unsafe {
        libc::waitid(
            libc::P_PID,
            process_id as libc::id_t,
            status.as_mut_ptr(),
            WAITID_OPTIONS,
        )
    };
    if result == libc::c_int::default() {
        return true;
    }
    let error = std::io::Error::last_os_error();
    if error.kind() == std::io::ErrorKind::Interrupted {
        return true;
    }
    assert_eq!(error.raw_os_error(), Some(libc::ECHILD));
    false
}
