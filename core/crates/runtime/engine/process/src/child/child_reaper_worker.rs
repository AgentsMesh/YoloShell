use super::{
    REAPER_POLL_INTERVAL, RetainedChild, Shared, TerminationPolicy, is_child_already_reaped,
    lock_queue,
};
use std::io;
use std::sync::Arc;
#[cfg(test)]
use std::sync::atomic::Ordering;

const REAPER_POLL_RETRY: &str = "detached child poll failed; retaining custody";
const REAPER_TERMINATION_RETRY: &str =
    "detached child termination failed; retaining termination policy";
#[cfg(test)]
const INJECTED_PAYLOAD_DROP_PANIC: &str =
    "injected detached child reaper worker payload Drop panic";

pub(super) fn run(shared: Arc<Shared>) {
    let mut retained = Vec::new();
    loop {
        let result = super::containment::catch_terminal(|| run_iteration(&shared, &mut retained));
        if result.is_err() {
            let mut queue = lock_queue(&shared);
            queue.worker_failed = true;
            drop(queue);
            shared.ready.notify_all();
            fallback(&shared, &mut retained);
        }
    }
}

fn fallback(
    shared: &Arc<Shared>,
    retained: &mut Vec<RetainedChild<super::super::DetachedChildCustody>>,
) -> ! {
    loop {
        let _ = super::containment::catch_terminal(|| run_iteration(shared, retained));
        std::thread::sleep(REAPER_POLL_INTERVAL);
    }
}

fn run_iteration(
    shared: &Arc<Shared>,
    retained: &mut Vec<RetainedChild<super::super::DetachedChildCustody>>,
) {
    receive(shared, retained);
    #[cfg(test)]
    if shared.panic_after_receive.swap(false, Ordering::SeqCst) {
        std::panic::panic_any(InjectedWorkerPanicPayload(Arc::clone(
            &shared.panic_payload_drops,
        )));
    }
    let progress = reap_retained_with_progress(retained);
    let mut queue = lock_queue(shared);
    queue.custody_count -= progress.settled;
    queue.termination_count -= progress.terminations;
    drop(queue);
    if progress.terminations > usize::default() {
        shared.ready.notify_all();
    }
}

fn receive(shared: &Shared, retained: &mut Vec<RetainedChild<super::super::DetachedChildCustody>>) {
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
                .wait_timeout(queue, REAPER_POLL_INTERVAL)
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            queue = next;
            break;
        }
    }
    retained.extend(queue.children.drain(..));
}

pub(in crate::process) trait ReapChild {
    fn process_id(&self) -> u32;
    fn try_reap(&mut self) -> io::Result<bool>;
    fn request_termination(&mut self) -> io::Result<()>;
}

impl ReapChild for super::super::DetachedChildCustody {
    fn process_id(&self) -> u32 {
        self.process_id()
    }

    fn try_reap(&mut self) -> io::Result<bool> {
        self.try_wait().map(|status| status.is_some())
    }

    fn request_termination(&mut self) -> io::Result<()> {
        self.request_direct_termination()
    }
}

#[cfg(test)]
pub(in crate::process) fn reap_retained<C: ReapChild>(
    children: &mut Vec<RetainedChild<C>>,
) -> usize {
    reap_retained_with_progress(children).settled
}

struct ReapProgress {
    settled: usize,
    terminations: usize,
}

fn reap_retained_with_progress<C: ReapChild>(children: &mut Vec<RetainedChild<C>>) -> ReapProgress {
    let initial = children.len();
    let pending = termination_count(children);
    let mut index = 0;
    while index < children.len() {
        if settle_or_retain(&mut children[index]) {
            children.swap_remove(index);
        } else {
            index += 1;
        }
    }
    ReapProgress {
        settled: initial - children.len(),
        terminations: pending - termination_count(children),
    }
}

fn termination_count<C>(children: &[RetainedChild<C>]) -> usize {
    children
        .iter()
        .filter(|child| child.policy == TerminationPolicy::Terminate)
        .count()
}

fn settle_or_retain<C: ReapChild>(retained: &mut RetainedChild<C>) -> bool {
    let process_id = retained.child.process_id();
    match retained.child.try_reap() {
        Ok(true) => return true,
        Err(error) if is_child_already_reaped(&error) => return true,
        Ok(false) => {}
        Err(error) => {
            super::containment::report_io_error(error, process_id, REAPER_POLL_RETRY);
        }
    }
    if retained.policy == TerminationPolicy::Terminate {
        match retained.child.request_termination() {
            Ok(()) => retained.policy = TerminationPolicy::NaturalExit,
            Err(error) if process_is_gone(&error) => {
                retained.policy = TerminationPolicy::NaturalExit;
            }
            Err(error) => {
                super::containment::report_io_error(error, process_id, REAPER_TERMINATION_RETRY);
            }
        }
    }
    false
}

fn process_is_gone(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ESRCH)
}

#[cfg(test)]
struct InjectedWorkerPanicPayload(Arc<std::sync::atomic::AtomicUsize>);

#[cfg(test)]
impl Drop for InjectedWorkerPanicPayload {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
        panic!("{INJECTED_PAYLOAD_DROP_PANIC}");
    }
}

#[cfg(test)]
#[cfg(any(not(test), feature = "yolo-integration-tests"))]
#[path = "../../integration_tests/child/child_reaper_worker_containment_tests.rs"]
mod containment_tests;
