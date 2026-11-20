use super::*;
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

const PENDING_FILE_NAME: &str = "pending.lock";
const PENDING_RESERVATION_UNAVAILABLE: &str = "shared kernel reservation unavailable";
const PENDING_WAITER_UNAVAILABLE: &str = "shared pending waiter unavailable";
const SHARED_WAITER_COUNT: usize = 2;
const TEST_TIMEOUT: Duration = Duration::from_secs(5);

#[test]
fn reservation_completion_wakes_every_shared_waiter() {
    let directory = tempfile::tempdir().unwrap();
    let file = pending_file(directory.path());
    let key = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let reservation = reserved_shared(key);
    let waiters: [wait_signal::ProcessWaitToken; SHARED_WAITER_COUNT] =
        std::array::from_fn(|_| pending_shared(key));
    let (completed, observed) = mpsc::channel();
    let threads: Vec<_> = waiters
        .into_iter()
        .map(|waiter| {
            let completed = completed.clone();
            std::thread::spawn(move || {
                waiter.wait();
                completed.send(()).unwrap();
            })
        })
        .collect();

    drop(reservation);
    for _ in &threads {
        observed.recv_timeout(TEST_TIMEOUT).unwrap();
    }
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn successful_install_wakes_every_waiter_created_before_notification() {
    let directory = tempfile::tempdir().unwrap();
    let file = pending_file(directory.path());
    let key = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let reservation = reserved_shared(key);
    let waiters: [wait_signal::ProcessWaitToken; SHARED_WAITER_COUNT] =
        std::array::from_fn(|_| pending_shared(key));

    let _installed = reservation.install(file.try_clone().unwrap()).unwrap();
    let (completed, observed) = mpsc::channel();
    let threads: Vec<_> = waiters
        .into_iter()
        .map(|waiter| {
            let completed = completed.clone();
            std::thread::spawn(move || {
                waiter.wait();
                completed.send(()).unwrap();
            })
        })
        .collect();

    for _ in &threads {
        observed.recv_timeout(TEST_TIMEOUT).unwrap();
    }
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn stale_pending_reservation_cannot_replace_its_successor() {
    let directory = tempfile::tempdir().unwrap();
    let file = pending_file(directory.path());
    let key = ObjectIdentity::from_metadata(&file.metadata().unwrap());
    let stale = reserved_shared(key);
    current_registry().entries.remove(&key);
    let successor = reserved_shared(key);

    assert!(stale.install(file.try_clone().unwrap()).is_err());
    assert!(matches!(
        registry::begin_kernel(key, Mode::Shared).unwrap(),
        registry::KernelAcquisition::PendingShared(_)
    ));
    drop(successor);
    assert!(matches!(
        registry::begin_kernel(key, Mode::Shared).unwrap(),
        registry::KernelAcquisition::Reserved(_)
    ));
}

fn reserved_shared(key: ObjectIdentity) -> registry::KernelReservation {
    match registry::begin_kernel(key, Mode::Shared).unwrap() {
        registry::KernelAcquisition::Reserved(reservation) => reservation,
        _ => panic!("{PENDING_RESERVATION_UNAVAILABLE}"),
    }
}

fn pending_shared(key: ObjectIdentity) -> wait_signal::ProcessWaitToken {
    match registry::begin_kernel(key, Mode::Shared).unwrap() {
        registry::KernelAcquisition::PendingShared(waiter) => waiter,
        _ => panic!("{PENDING_WAITER_UNAVAILABLE}"),
    }
}

fn pending_file(directory: &Path) -> std::fs::File {
    OpenOptions::new()
        .create_new(true)
        .read(true)
        .write(true)
        .open(directory.join(PENDING_FILE_NAME))
        .unwrap()
}
