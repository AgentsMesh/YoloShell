use super::*;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const TEST_EXIT_CODE: u32 = 0;
const CALLBACK_INCREMENT: usize = 1;

#[derive(Debug)]
struct UnidentifiedSpawnChild {
    terminated: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
    signals: Arc<AtomicUsize>,
}

impl Drop for UnidentifiedSpawnChild {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::Release);
    }
}

impl portable_pty::ChildKiller for UnidentifiedSpawnChild {
    fn kill(&mut self) -> io::Result<()> {
        self.signals.fetch_add(CALLBACK_INCREMENT, Ordering::AcqRel);
        self.terminated.store(true, Ordering::Release);
        Ok(())
    }

    fn clone_killer(&self) -> Box<dyn portable_pty::ChildKiller + Send + Sync> {
        Box::new(Self {
            terminated: Arc::clone(&self.terminated),
            dropped: Arc::clone(&self.dropped),
            signals: Arc::clone(&self.signals),
        })
    }
}

impl portable_pty::Child for UnidentifiedSpawnChild {
    fn try_wait(&mut self) -> io::Result<Option<portable_pty::ExitStatus>> {
        Ok(self
            .terminated
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

#[tokio::test]
async fn identity_failure_settles_fresh_child_before_launch_returns() {
    let terminated = Arc::new(AtomicBool::default());
    let dropped = Arc::new(AtomicBool::default());
    let signals = Arc::new(AtomicUsize::default());
    let child = UnidentifiedSpawnChild {
        terminated: Arc::clone(&terminated),
        dropped: Arc::clone(&dropped),
        signals: Arc::clone(&signals),
    };
    let mut cleanup_scope = StartupCleanupScope::new().unwrap();
    let spawn_guard = process::waitable_child_spawn_guard().unwrap();
    let boundary = spawn_guard.boundary();

    let result = spawn_with_reserved_reaper(
        &mut cleanup_scope,
        || Ok(Box::new(child)),
        &boundary,
        ExclusivePtyOutput::observed(Arc::new(AtomicUsize::default())),
        UnreleasedBootstrapGate,
    );

    assert!(result.is_err());
    assert!(terminated.load(Ordering::Acquire));
    assert!(dropped.load(Ordering::Acquire));
    assert_eq!(signals.load(Ordering::Acquire), CALLBACK_INCREMENT);
    cleanup_scope.settle().await.unwrap();
}
