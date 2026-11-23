use super::support::ChildObservation;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const TEST_EXIT_CODE: u32 = 0;
const CALLBACK_INCREMENT: usize = 1;
const FIRST_POLL: usize = 0;
const SECOND_POLL: usize = 1;
const THIRD_POLL: usize = 2;
const POLL_PANIC_DETAIL: &str = "injected untracked reaper poll panic";

#[derive(Debug)]
pub(super) struct ScriptedChild {
    dropped: Arc<AtomicBool>,
    polls: Arc<AtomicUsize>,
    signals: Arc<AtomicUsize>,
}

#[derive(Debug)]
struct ScriptedKiller {
    signals: Arc<AtomicUsize>,
}

impl portable_pty::ChildKiller for ScriptedKiller {
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

impl ScriptedChild {
    pub(super) fn observed(observation: &ChildObservation) -> Self {
        Self {
            dropped: Arc::clone(&observation.dropped),
            polls: Arc::clone(&observation.polls),
            signals: Arc::clone(&observation.signals),
        }
    }
}

impl Drop for ScriptedChild {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

impl portable_pty::ChildKiller for ScriptedChild {
    fn kill(&mut self) -> io::Result<()> {
        self.signals.fetch_add(CALLBACK_INCREMENT, Ordering::AcqRel);
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(ScriptedKiller {
            signals: Arc::clone(&self.signals),
        })
    }
}

impl portable_pty::Child for ScriptedChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        match self.polls.fetch_add(CALLBACK_INCREMENT, Ordering::AcqRel) {
            FIRST_POLL => Err(io::ErrorKind::Interrupted.into()),
            SECOND_POLL => panic!("{POLL_PANIC_DETAIL}"),
            THIRD_POLL => Ok(None),
            _ => Ok(Some(portable_pty::ExitStatus::with_exit_code(
                TEST_EXIT_CODE,
            ))),
        }
    }

    fn wait(&mut self) -> io::Result<portable_pty::ExitStatus> {
        Ok(portable_pty::ExitStatus::with_exit_code(TEST_EXIT_CODE))
    }

    fn process_id(&self) -> Option<u32> {
        None
    }
}
