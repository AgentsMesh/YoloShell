use super::*;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::time::{Duration, Instant};

const EXPECTED_POLL_ATTEMPTS: usize = 2;
const NO_CALLBACKS: usize = 0;
const TEST_EXIT_CODE: u32 = 0;
const CUSTODY_PROBE: Duration = Duration::from_millis(50);
const CUSTODY_SETTLEMENT_TIMEOUT: Duration = Duration::from_secs(1);
const IDENTITY_PANIC_DETAIL: &str = "injected startup child identity panic";
const POLL_PANIC_DETAIL: &str = "injected startup child poll panic";
const STALE_CHILD_CALLBACK_DETAIL: &str = "stale child callback must not run after ECHILD";
const UNRELATED_SIGNAL_DETAIL: &str = "unrelated replacement signal must not run";

#[derive(Debug)]
struct UnidentifiedChild {
    polls: Arc<AtomicUsize>,
    signals: Arc<AtomicUsize>,
    first_poll: FirstPoll,
    released: Option<Arc<AtomicBool>>,
}

#[derive(Clone, Copy, Debug)]
enum FirstPoll {
    Error,
    Panic,
    Pending,
}

impl portable_pty::ChildKiller for UnidentifiedChild {
    fn kill(&mut self) -> io::Result<()> {
        self.signals.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self {
            polls: Arc::clone(&self.polls),
            signals: Arc::clone(&self.signals),
            first_poll: self.first_poll,
            released: self.released.clone(),
        })
    }
}

impl portable_pty::Child for UnidentifiedChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        let attempt = self.polls.fetch_add(1, Ordering::AcqRel);
        if attempt == 0 {
            match self.first_poll {
                FirstPoll::Error => return Err(io::ErrorKind::BrokenPipe.into()),
                FirstPoll::Panic => panic!("{POLL_PANIC_DETAIL}"),
                FirstPoll::Pending => {}
            }
        }
        let settled = self
            .released
            .as_ref()
            .is_none_or(|released| released.load(Ordering::Acquire));
        Ok(settled.then(|| portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE)))
    }

    fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        panic!("{POLL_PANIC_DETAIL}")
    }

    fn process_id(&self) -> Option<u32> {
        panic!("{IDENTITY_PANIC_DETAIL}")
    }
}

#[test]
fn transient_poll_error_retains_unidentified_child_without_signaling() {
    let (guard, polls, signals) = guard(FirstPoll::Error, None);
    drop(guard);
    wait_for_polls(&polls);
    assert_eq!(polls.load(Ordering::Acquire), EXPECTED_POLL_ATTEMPTS);
    assert_eq!(signals.load(Ordering::Acquire), NO_CALLBACKS);
}

#[test]
fn panicking_poll_is_contained_without_using_numeric_killer() {
    let (guard, polls, signals) = guard(FirstPoll::Panic, None);
    drop(guard);
    wait_for_polls(&polls);
    assert_eq!(polls.load(Ordering::Acquire), EXPECTED_POLL_ATTEMPTS);
    assert_eq!(signals.load(Ordering::Acquire), NO_CALLBACKS);
}

#[test]
fn unidentified_cleanup_withholds_return_until_child_settles() {
    let released = Arc::new(AtomicBool::new(false));
    let (mut guard, _polls, signals) = guard(FirstPoll::Pending, Some(Arc::clone(&released)));
    let (sender, receiver) = std::sync::mpsc::channel();
    let cleanup = std::thread::spawn(move || sender.send(guard.terminate()).unwrap());

    assert!(matches!(
        receiver.recv_timeout(CUSTODY_PROBE),
        Err(RecvTimeoutError::Timeout)
    ));
    assert_eq!(signals.load(Ordering::Acquire), NO_CALLBACKS);
    released.store(true, Ordering::Release);
    assert!(
        receiver
            .recv_timeout(CUSTODY_SETTLEMENT_TIMEOUT)
            .unwrap()
            .is_err()
    );
    cleanup.join().unwrap();
    assert_eq!(signals.load(Ordering::Acquire), NO_CALLBACKS);
}

#[test]
fn stale_child_pid_never_authorizes_replacement_signal_or_wait() {
    let callbacks = Arc::new(AtomicUsize::new(0));
    let signal_attempts = Arc::new(AtomicUsize::new(0));
    let signal_counter = Arc::clone(&signal_attempts);
    let _signal_failure = crate::process::install_test_live_signal_failure(move || {
        signal_counter.fetch_add(1, Ordering::AcqRel);
        Some(UNRELATED_SIGNAL_DETAIL.to_owned())
    });
    let child = StaleChild {
        callbacks: Arc::clone(&callbacks),
        stale_pid: std::process::id(),
    };

    drop(StartupChildGuard::new(Box::new(child)));

    assert_eq!(callbacks.load(Ordering::Acquire), NO_CALLBACKS);
    assert_eq!(signal_attempts.load(Ordering::Acquire), NO_CALLBACKS);
}

#[derive(Debug)]
struct StaleChild {
    callbacks: Arc<AtomicUsize>,
    stale_pid: u32,
}

impl portable_pty::ChildKiller for StaleChild {
    fn kill(&mut self) -> io::Result<()> {
        self.fail_callback()
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self {
            callbacks: Arc::clone(&self.callbacks),
            stale_pid: self.stale_pid,
        })
    }
}

impl portable_pty::Child for StaleChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        self.fail_callback()
    }

    fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        self.fail_callback()
    }

    fn process_id(&self) -> Option<u32> {
        Some(self.stale_pid)
    }
}

impl StaleChild {
    fn fail_callback<T>(&self) -> T {
        self.callbacks.fetch_add(1, Ordering::AcqRel);
        panic!("{STALE_CHILD_CALLBACK_DETAIL}")
    }
}

fn guard(
    first_poll: FirstPoll,
    released: Option<Arc<AtomicBool>>,
) -> (StartupChildGuard, Arc<AtomicUsize>, Arc<AtomicUsize>) {
    let polls = Arc::new(AtomicUsize::new(0));
    let signals = Arc::new(AtomicUsize::new(0));
    let child = UnidentifiedChild {
        polls: Arc::clone(&polls),
        signals: Arc::clone(&signals),
        first_poll,
        released,
    };
    (StartupChildGuard::new(Box::new(child)), polls, signals)
}

fn wait_for_polls(polls: &AtomicUsize) {
    let deadline = Instant::now() + CUSTODY_SETTLEMENT_TIMEOUT;
    while polls.load(Ordering::Acquire) != EXPECTED_POLL_ATTEMPTS {
        assert!(Instant::now() < deadline);
        std::thread::sleep(CUSTODY_PROBE);
    }
}
