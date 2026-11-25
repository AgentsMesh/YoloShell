use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const TEST_EXIT_CODE: u32 = 0;
const CALLBACK_INCREMENT: usize = 1;

#[derive(Clone)]
pub(super) struct ChildObservation {
    pub(super) settled: Arc<AtomicBool>,
    pub(super) dropped: Arc<AtomicBool>,
    pub(super) polls: Arc<AtomicUsize>,
    pub(super) signals: Arc<AtomicUsize>,
}

impl ChildObservation {
    pub(super) fn new() -> Self {
        Self {
            settled: Arc::new(AtomicBool::new(false)),
            dropped: Arc::new(AtomicBool::new(false)),
            polls: Arc::new(AtomicUsize::new(usize::default())),
            signals: Arc::new(AtomicUsize::new(usize::default())),
        }
    }
}

#[derive(Debug)]
struct ObservedKiller {
    signals: Arc<AtomicUsize>,
}

impl portable_pty::ChildKiller for ObservedKiller {
    fn kill(&mut self) -> io::Result<()> {
        self.signals.fetch_add(CALLBACK_INCREMENT, Ordering::AcqRel);
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self {
            signals: Arc::clone(&self.signals),
        })
    }
}

#[derive(Debug)]
pub(super) struct GatedChild {
    settled: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
    polls: Arc<AtomicUsize>,
    killer: ObservedKiller,
}

impl GatedChild {
    pub(super) fn observed(observation: &ChildObservation) -> Self {
        Self {
            settled: Arc::clone(&observation.settled),
            dropped: Arc::clone(&observation.dropped),
            polls: Arc::clone(&observation.polls),
            killer: ObservedKiller {
                signals: Arc::clone(&observation.signals),
            },
        }
    }
}

impl Drop for GatedChild {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

impl portable_pty::ChildKiller for GatedChild {
    fn kill(&mut self) -> io::Result<()> {
        self.killer.kill()
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        self.killer.clone_killer()
    }
}

impl portable_pty::Child for GatedChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        self.polls.fetch_add(CALLBACK_INCREMENT, Ordering::AcqRel);
        Ok(self
            .settled
            .load(Ordering::Acquire)
            .then(|| portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE)))
    }

    fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        Ok(portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE))
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}
