use super::*;
use crate::process::{DetachedStandardError, is_process_alive, spawn_detached};
use std::path::Path;
use std::sync::{Arc, Mutex};

const CHILD_EXECUTABLE: &str = "/bin/sleep";
const CHILD_DURATION: &str = "30";
const EXPECTED_CHILD_SPAWNS: usize = 0;
const REAPER_LAUNCH_FAILURE: &str = "injected detached child reaper launch failure";
const REAPER_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Debug)]
struct StructuredReservationFailure;

impl std::fmt::Display for StructuredReservationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(REAPER_LAUNCH_FAILURE)
    }
}

impl std::error::Error for StructuredReservationFailure {}

#[test]
fn reaper_initialization_failure_precedes_any_child_spawn() {
    let child_spawns = std::sync::atomic::AtomicUsize::new(usize::default());
    let result = DetachedChildReaper::start_with(|_| Err(io::Error::other(REAPER_LAUNCH_FAILURE)));
    if result.is_ok() {
        child_spawns.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    assert_eq!(result.err().unwrap().to_string(), REAPER_LAUNCH_FAILURE);
    assert_eq!(
        child_spawns.load(std::sync::atomic::Ordering::SeqCst),
        EXPECTED_CHILD_SPAWNS
    );
}

#[test]
fn failed_reservation_preserves_the_structured_cause_and_exact_child() {
    let child = spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let process_id = child.id();

    let failure =
        handoff_with_reservation(child, Err(anyhow::Error::new(StructuredReservationFailure)))
            .unwrap_err();

    assert_eq!(
        std::error::Error::source(&failure)
            .unwrap()
            .downcast_ref::<StructuredReservationFailure>()
            .unwrap()
            .to_string(),
        REAPER_LAUNCH_FAILURE
    );
    let (source, mut preserved) = failure.into_parts();
    assert!(
        source
            .downcast_ref::<StructuredReservationFailure>()
            .is_some()
    );
    assert_eq!(preserved.id(), process_id);
    preserved.kill().unwrap();
    preserved.wait().unwrap();
}

#[test]
fn reserved_settlement_survives_capacity_saturation_until_exact_reap() {
    let (entered_send, entered_receive) = std::sync::mpsc::channel();
    let (release_send, release_receive) = std::sync::mpsc::channel();
    let release_receive = Arc::new(Mutex::new(release_receive));
    let reaper = DetachedChildReaper::start_with(|shared| {
        std::thread::spawn(move || {
            let child = {
                let mut queue = lock_queue(&shared);
                while queue.children.is_empty() {
                    queue = shared
                        .ready
                        .wait(queue)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                }
                queue.children.pop_front().unwrap().child
            };
            entered_send.send(child.process_id()).unwrap();
            release_receive.lock().unwrap().recv().unwrap();
            let mut child = child;
            child.request_direct_termination().unwrap();
            child.wait().unwrap();
            let mut queue = lock_queue(&shared);
            queue.custody_count -= 1;
            queue.termination_count -= 1;
            drop(queue);
            shared.ready.notify_all();
        });
        Ok(())
    })
    .unwrap();
    let reservation = reaper.reserve().unwrap();
    let retained = (1..REAPER_QUEUE_CAPACITY)
        .map(|_| reaper.reserve().unwrap())
        .collect::<Vec<_>>();
    assert!(reaper.reserve().is_err());
    let child = spawn_detached(
        Path::new(CHILD_EXECUTABLE),
        &[CHILD_DURATION.into()],
        DetachedStandardError::Null,
    )
    .unwrap();
    let process_id = child.id();

    reserved::ReservedDetachedChildSettlement::new(reservation).terminate(child);
    assert_eq!(entered_receive.recv().unwrap(), process_id);
    let barrier = reaper.clone();
    let (barrier_send, barrier_receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        barrier
            .wait_for_termination_delivery(std::time::Instant::now() + REAPER_WAIT_TIMEOUT)
            .unwrap();
        barrier_send.send(()).unwrap();
    });
    assert!(reaper.reserve().is_err());
    assert!(is_process_alive(process_id));
    assert!(barrier_receive.try_recv().is_err());

    release_send.send(()).unwrap();
    while is_process_alive(process_id) {
        std::thread::yield_now();
    }
    barrier_receive.recv().unwrap();
    drop(retained);
    assert!(reaper.reserve().is_ok());
}
