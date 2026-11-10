#[path = "../test_support/constants.rs"]
mod constants;
use constants::*;
use std::{io::Write, os::unix::net::UnixStream, sync::mpsc, time::Instant};
use yolo_hub_file_helper::wait_for_owner_loss;

#[test]
fn owner_close_and_unexpected_data_end_the_wait_without_waiting_for_a_request() {
    for data in [false, true] {
        let (mut owner, helper) = UnixStream::pair().unwrap();
        let (done, finished) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            wait_for_owner_loss(helper, Instant::now() + DEADLINE);
            done.send(()).unwrap();
        });
        if data {
            owner.write_all(INVALID_OWNER_DATA).unwrap();
        } else {
            drop(owner);
        }
        finished.recv_timeout(DEADLINE).unwrap();
        thread.join().unwrap();
    }
}

#[test]
fn an_alive_owner_cannot_extend_an_expired_watchdog_budget() {
    for duration in [std::time::Duration::ZERO, SHORT_DEADLINE] {
        let (_owner, helper) = UnixStream::pair().unwrap();
        let started = Instant::now();
        wait_for_owner_loss(helper, started + duration);
        assert!(started.elapsed() < DEADLINE);
    }
}
