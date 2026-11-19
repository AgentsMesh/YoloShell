use super::*;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

const EXPECTED_POLL_ATTEMPTS: usize = 1;
const EXPECTED_SIGNAL_ATTEMPTS: usize = 0;
const TEST_EXIT_CODE: u32 = 0;
const DROP_RETURN_DEADLINE: Duration = Duration::from_millis(250);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
const CLEANUP_POLL_INTERVAL: Duration = Duration::from_millis(5);

struct ObservedMaster(Arc<AtomicBool>);

impl Drop for ObservedMaster {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

#[derive(Debug)]
struct OrderedChild {
    master_dropped: Arc<AtomicBool>,
    master_dropped_during_poll: Arc<AtomicBool>,
    polls: Arc<AtomicUsize>,
    signals: Arc<AtomicUsize>,
}

impl portable_pty::ChildKiller for OrderedChild {
    fn kill(&mut self) -> io::Result<()> {
        self.signals.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self {
            master_dropped: Arc::clone(&self.master_dropped),
            master_dropped_during_poll: Arc::clone(&self.master_dropped_during_poll),
            polls: Arc::clone(&self.polls),
            signals: Arc::clone(&self.signals),
        })
    }
}

impl portable_pty::Child for OrderedChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        self.master_dropped_during_poll.store(
            self.master_dropped.load(Ordering::Acquire),
            Ordering::Release,
        );
        self.polls.fetch_add(1, Ordering::AcqRel);
        Ok(Some(portable_pty::ExitStatus::with_exit_code(
            TEST_EXIT_CODE,
        )))
    }

    fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        Ok(portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE))
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}

#[test]
fn startup_resource_drop_hands_off_output_before_releasing_master() {
    let master_dropped = Arc::new(AtomicBool::new(false));
    let progress_observed = Arc::new(AtomicUsize::new(usize::default()));
    let master_dropped_during_poll = Arc::new(AtomicBool::new(false));
    let polls = Arc::new(AtomicUsize::new(0));
    let signals = Arc::new(AtomicUsize::new(0));
    let child = OrderedChild {
        master_dropped: Arc::clone(&master_dropped),
        master_dropped_during_poll: Arc::clone(&master_dropped_during_poll),
        polls: Arc::clone(&polls),
        signals: Arc::clone(&signals),
    };
    let output = ExclusivePtyOutput::observed(Arc::clone(&progress_observed));
    let resources = StartupPtyResources::with_output(
        ObservedMaster(Arc::clone(&master_dropped)),
        Box::new(child),
        output,
    );

    let started = Instant::now();
    drop(resources);

    assert!(started.elapsed() < DROP_RETURN_DEADLINE);
    assert!(master_dropped.load(Ordering::Acquire));
    wait_until(|| polls.load(Ordering::Acquire) == EXPECTED_POLL_ATTEMPTS);
    assert!(progress_observed.load(Ordering::Acquire) > usize::default());
    assert!(master_dropped_during_poll.load(Ordering::Acquire));
    assert_eq!(polls.load(Ordering::Acquire), EXPECTED_POLL_ATTEMPTS);
    assert_eq!(signals.load(Ordering::Acquire), EXPECTED_SIGNAL_ATTEMPTS);
}

#[test]
fn startup_resource_without_output_authority_releases_master_before_child_cleanup() {
    let master_dropped = Arc::new(AtomicBool::new(false));
    let master_dropped_during_poll = Arc::new(AtomicBool::new(false));
    let polls = Arc::new(AtomicUsize::new(0));
    let signals = Arc::new(AtomicUsize::new(0));
    let child = OrderedChild {
        master_dropped: Arc::clone(&master_dropped),
        master_dropped_during_poll: Arc::clone(&master_dropped_during_poll),
        polls: Arc::clone(&polls),
        signals: Arc::clone(&signals),
    };
    let resources =
        StartupPtyResources::new(ObservedMaster(Arc::clone(&master_dropped)), Box::new(child));

    let started = Instant::now();
    drop(resources);

    assert!(started.elapsed() < DROP_RETURN_DEADLINE);
    assert!(master_dropped.load(Ordering::Acquire));
    wait_until(|| polls.load(Ordering::Acquire) == EXPECTED_POLL_ATTEMPTS);
    assert!(master_dropped_during_poll.load(Ordering::Acquire));
    assert_eq!(polls.load(Ordering::Acquire), EXPECTED_POLL_ATTEMPTS);
    assert_eq!(signals.load(Ordering::Acquire), EXPECTED_SIGNAL_ATTEMPTS);
}

fn wait_until(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + CLEANUP_TIMEOUT;
    while !predicate() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(CLEANUP_POLL_INTERVAL);
    }
}
