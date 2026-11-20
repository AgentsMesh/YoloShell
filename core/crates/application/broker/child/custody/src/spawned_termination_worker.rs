use super::{RetainedChild, Shared, containment, lock_queue};
use std::io;
use std::sync::Arc;

const POLL_FAILURE: &str = "broker cleanup child poll failed; retaining custody";
const TERMINATION_FAILURE: &str = "broker cleanup child termination failed; retaining custody";

pub(super) fn run(shared: Arc<Shared>) {
    let mut retained = Vec::new();
    loop {
        let result = containment::catch_unwind(|| run_iteration(&shared, &mut retained));
        if result.is_err() {
            let mut queue = lock_queue(&shared);
            queue.worker_failed = true;
            drop(queue);
            shared.ready.notify_all();
            fallback(&shared, &mut retained);
        }
    }
}

fn fallback(shared: &Arc<Shared>, retained: &mut Vec<RetainedChild>) -> ! {
    loop {
        let _ = containment::catch_unwind(|| run_iteration(shared, retained));
        std::thread::sleep(yolo_remote_broker::BROKER_READY_POLL_INTERVAL);
    }
}

fn run_iteration(shared: &Arc<Shared>, retained: &mut Vec<RetainedChild>) {
    receive(shared, retained);
    let progress = advance(retained);
    let mut queue = lock_queue(shared);
    queue.custody_count -= progress.settled;
    queue.termination_count -= progress.terminations;
    drop(queue);
    if progress.terminations > usize::default() || progress.settled > usize::default() {
        shared.ready.notify_all();
    }
}

fn receive(shared: &Shared, retained: &mut Vec<RetainedChild>) {
    let mut queue = lock_queue(shared);
    while queue.children.is_empty() {
        if retained.is_empty() {
            queue = shared
                .ready
                .wait(queue)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
        } else {
            let (next, _) = shared
                .ready
                .wait_timeout(queue, yolo_remote_broker::BROKER_READY_POLL_INTERVAL)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            queue = next;
            break;
        }
    }
    retained.extend(queue.children.drain(..));
}

pub(super) trait ReapChild {
    fn process_id(&self) -> u32;
    fn try_reap(&mut self) -> io::Result<bool>;
    fn terminate(&mut self) -> io::Result<()>;
}

impl ReapChild for yolo_process::DetachedChildCustody {
    fn process_id(&self) -> u32 {
        self.process_id()
    }

    fn try_reap(&mut self) -> io::Result<bool> {
        self.try_wait().map(|status| status.is_some())
    }

    fn terminate(&mut self) -> io::Result<()> {
        self.request_direct_termination()
    }
}

pub(super) struct Progress {
    pub(super) settled: usize,
    pub(super) terminations: usize,
}

pub(super) fn advance<C: ReapChild>(children: &mut Vec<RetainedChild<C>>) -> Progress {
    let mut progress = Progress {
        settled: usize::default(),
        terminations: usize::default(),
    };
    let mut index = usize::default();
    while index < children.len() {
        let child = &mut children[index];
        let process_id = containment::catch_unwind(|| child.child.process_id()).unwrap_or_default();
        match child.child.try_reap() {
            Ok(true) => {
                if child.termination_required {
                    progress.terminations += 1;
                }
                children.swap_remove(index);
                progress.settled += 1;
                continue;
            }
            Err(error) if yolo_process::is_child_already_reaped(&error) => {
                containment::discard(error);
                if child.termination_required {
                    progress.terminations += 1;
                }
                children.swap_remove(index);
                progress.settled += 1;
                continue;
            }
            Err(error) => containment::report(error, process_id, POLL_FAILURE),
            Ok(false) => {}
        }
        if child.termination_required {
            match child.child.terminate() {
                Ok(()) => {
                    child.termination_required = false;
                    progress.terminations += 1;
                }
                Err(error) if process_is_gone(&error) => {
                    containment::discard(error);
                    child.termination_required = false;
                    progress.terminations += 1;
                }
                Err(error) => containment::report(error, process_id, TERMINATION_FAILURE),
            }
        }
        index += 1;
    }
    progress
}

fn process_is_gone(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ESRCH)
}
