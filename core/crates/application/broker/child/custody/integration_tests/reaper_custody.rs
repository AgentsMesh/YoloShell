use super::*;
use std::process::Command;
use std::sync::{Arc, Mutex};
const CHILD_COMMAND: &str = "sleep 30";
const CHILD_SHELL: &str = "/bin/sh";
const CHILD_SHELL_ARGUMENT: &str = "-c";
const REAPER_WAIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1);

#[test]
fn blocked_worker_retains_exact_custody_and_keeps_queue_bounded() {
    let (entered_send, entered_receive) = std::sync::mpsc::channel();
    let (release_send, release_receive) = std::sync::mpsc::channel();
    let (settled_send, settled_receive) = std::sync::mpsc::channel();
    let release_receive = Arc::new(Mutex::new(release_receive));
    let reaper = TerminationReaper::start_with(|shared| {
        std::thread::spawn(move || {
            let retained = {
                let mut queue = lock_queue(&shared);
                while queue.children.is_empty() {
                    queue = shared
                        .ready
                        .wait(queue)
                        .unwrap_or_else(std::sync::PoisonError::into_inner);
                }
                queue.children.pop_front().unwrap()
            };
            let mut child = retained.child;
            entered_send.send(child.process_id()).unwrap();
            release_receive.lock().unwrap().recv().unwrap();
            let _ = child.request_direct_termination();
            let _ = child.wait();
            settled_send.send(()).unwrap();
            let mut queue = lock_queue(&shared);
            queue.custody_count -= 1;
            queue.termination_count -= usize::from(retained.termination_required);
            drop(queue);
            shared.ready.notify_all();
        });
        Ok(())
    })
    .unwrap();
    let reservation = reaper.reserve().unwrap();
    let child = Command::new(CHILD_SHELL)
        .args([CHILD_SHELL_ARGUMENT, CHILD_COMMAND])
        .spawn()
        .unwrap();
    let process_id = child.id();
    let child = yolo_process::DetachedChildCustody::capture(child).unwrap();

    reservation.retain(child, true);
    assert_eq!(entered_receive.recv().unwrap(), process_id);
    let barrier = reaper.clone();
    let (barrier_send, barrier_receive) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        barrier
            .wait_for_termination_delivery(std::time::Instant::now() + REAPER_WAIT_TIMEOUT)
            .unwrap();
        barrier_send.send(()).unwrap();
    });
    let retained = (1..TERMINATION_QUEUE_CAPACITY)
        .map(|_| reaper.reserve().unwrap())
        .collect::<Vec<_>>();
    assert!(reaper.reserve().is_err());
    assert!(yolo_process::is_process_alive(process_id));
    assert!(barrier_receive.try_recv().is_err());

    release_send.send(()).unwrap();
    settled_receive.recv().unwrap();
    barrier_receive.recv().unwrap();
    drop(retained);
    assert!(reaper.reserve().is_ok());
    assert!(!yolo_process::is_process_alive(process_id));
}
